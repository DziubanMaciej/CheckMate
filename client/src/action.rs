use crate::config::Config;
use check_mate_common::{CommunicationError, ServerCommand, DEFAULT_WATCH_INTERVAL};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncWrite};

#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages(bool),
    WatchCommand(WatchCommandData),
    RefreshClientByName(String),
    Abort,
}

#[derive(PartialEq, Debug)]
pub struct WatchCommandData {
    pub command: String,
    pub command_args: Vec<String>,
    pub interval: Duration,
}

impl WatchCommandData {
    pub fn new(command: String, command_args: Vec<String>) -> Self {
        Self {
            command,
            command_args,
            interval: DEFAULT_WATCH_INTERVAL,
        }
    }
}

impl Action {
    pub fn should_reconnect(&self) -> bool {
        match self {
            Self::WatchCommand(_) => true,
            _ => false,
        }
    }

    pub async fn execute(
        &self,
        input_stream: &mut (impl AsyncBufRead + Unpin),
        output_stream: &mut (impl AsyncWrite + Unpin),
        config: &Config,
    ) -> Result<(), CommunicationError> {
        if let Some(ref name) = config.client_name {
            let command = ServerCommand::SetName(name.clone());
            command.send_async(output_stream).await?;
        }

        match self {
            Action::ReadMessages(include_names) => {
                Self::read(input_stream, output_stream, *include_names).await
            }
            Action::WatchCommand(data) => {
                Self::watch(
                    input_stream,
                    output_stream,
                    &data.command,
                    &data.command_args,
                    data.interval,
                )
                .await
            }
            Action::RefreshClientByName(name) => {
                Self::refresh_client_by_name(output_stream, name).await
            }
            Action::Abort => Self::abort(output_stream).await,
        }
    }

    async fn read(
        input_stream: &mut (impl AsyncBufRead + Unpin),
        output_stream: &mut (impl AsyncWrite + Unpin),
        include_names: bool,
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::GetStatuses(include_names);
        command.send_async(output_stream).await?;

        match ServerCommand::receive_async(input_stream).await? {
            ServerCommand::Statuses(statuses) => {
                for status in statuses.iter() {
                    println!("{}", status);
                }
            }
            _ => panic!("Unexpected command received after GetStatuses"),
        }
        Ok(())
    }

    async fn watch(
        input_stream: &mut (impl AsyncBufRead + Unpin),
        output_stream: &mut (impl AsyncWrite + Unpin),
        command: &str,
        command_args: &Vec<String>,
        interval: Duration,
    ) -> Result<(), CommunicationError> {
        async fn do_watch(
            output_stream: &mut (impl AsyncWrite + Unpin),
            command: &str,
            command_args: &Vec<String>,
        ) -> Result<(), CommunicationError> {
            // Run command to get its output
            let command = command.to_string();
            let command_args = command_args.clone();
            let command_output = tokio::task::spawn_blocking(move || {
                Action::execute_command(&command, &command_args)
            })
            .await;
            let command_output = command_output.expect("JoinError is unexpected for watch");
            let command_output = command_output
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(1)
                .next()
                .unwrap_or("")
                .to_string();

            // Send status to the server
            let server_command = if command_output.is_empty() {
                ServerCommand::SetStatusOk
            } else {
                ServerCommand::SetStatusError(command_output)
            };
            server_command.send_async(output_stream).await?;

            Ok(())
        }

        // Run first iteration immediately
        do_watch(output_stream, command, command_args).await?;

        loop {
            // Wait for either watch interval or refresh signal from server
            tokio::select! {
                _ = tokio::time::sleep(interval) => (),
                server_command = ServerCommand::receive_async(input_stream) => {
                    match server_command? {
                        ServerCommand::Refresh => (),
                        _ => panic!("Unexpected command received during watch"),
                    }
                }
            }

            // Execute command
            do_watch(output_stream, command, command_args).await?;
        }
    }

    async fn refresh_client_by_name(
        output_stream: &mut (impl AsyncWrite + Unpin),
        name: &str,
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::RefreshClientByName(name.into());
        command.send_async(output_stream).await
    }

    async fn abort(
        output_stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::Abort;
        command.send_async(output_stream).await
    }

    fn execute_command(command: &str, command_args: &Vec<String>) -> String {
        // TODO convert to async
        let subprocess = std::process::Command::new(command)
            .args(command_args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        let subprocess = match subprocess {
            Ok(x) => x,
            Err(err) => return err.to_string(),
        };

        let subprocess_result = subprocess.wait_with_output();
        let subprocess_result = match subprocess_result {
            Ok(x) => x,
            Err(err) => return err.to_string(),
        };

        let subprocess_out = if subprocess_result.status.success() {
            subprocess_result.stdout
        } else {
            subprocess_result.stderr
        };

        let subprocess_out = String::from_utf8(subprocess_out);
        let subprocess_out = match subprocess_out {
            Ok(x) => x,
            Err(err) => return err.to_string(),
        };

        subprocess_out
    }
}
