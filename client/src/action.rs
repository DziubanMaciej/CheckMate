use crate::config::Config;
use check_mate_common::{CommunicationError, ServerCommand, DEFAULT_SHELL, DEFAULT_WATCH_INTERVAL};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncWrite};

#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages(bool),
    WatchCommand(WatchCommandData),
    RefreshClientByName(String),
    RefreshAllClients,
    Abort,
    Help,
}

#[derive(PartialEq, Debug)]
pub struct WatchCommandData {
    pub command: String,
    pub command_args: Vec<String>,
    pub interval: Duration,
    pub shell: bool,
}

impl WatchCommandData {
    pub fn new(command: String, command_args: Vec<String>) -> Self {
        Self {
            command,
            command_args,
            interval: DEFAULT_WATCH_INTERVAL,
            shell: DEFAULT_SHELL,
        }
    }
}

impl Action {
    pub fn should_reconnect(&self) -> bool {
        matches!(self, Self::WatchCommand(_))
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
            Action::WatchCommand(data) => Self::watch(input_stream, output_stream, data).await,
            Action::RefreshClientByName(name) => {
                Self::refresh_client_by_name(output_stream, name).await
            }
            Action::RefreshAllClients => Self::refresh_all_clients(output_stream).await,
            Action::Abort => Self::abort(output_stream).await,
            Action::Help => panic!("Cannot execute help action"),
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
        data: &WatchCommandData,
    ) -> Result<(), CommunicationError> {
        async fn do_watch(
            output_stream: &mut (impl AsyncWrite + Unpin),
            data: &WatchCommandData,
        ) -> Result<(), CommunicationError> {
            // Run command to get its output
            let command = data.command.to_string();
            let command_args = data.command_args.to_owned();
            let command_output = Action::execute_command(&command, &command_args, data.shell).await;
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
        do_watch(output_stream, data).await?;

        loop {
            // Wait for either watch interval or refresh signal from server
            tokio::select! {
                _ = tokio::time::sleep(data.interval) => (),
                server_command = ServerCommand::receive_async(input_stream) => {
                    match server_command? {
                        ServerCommand::Refresh => (),
                        _ => panic!("Unexpected command received during watch"),
                    }
                }
            }

            // Execute command
            do_watch(output_stream, data).await?;
        }
    }

    async fn refresh_client_by_name(
        output_stream: &mut (impl AsyncWrite + Unpin),
        name: &str,
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::RefreshClientByName(name.into());
        command.send_async(output_stream).await
    }

    async fn refresh_all_clients(
        output_stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::RefreshAllClients;
        command.send_async(output_stream).await
    }

    async fn abort(
        output_stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::Abort;
        command.send_async(output_stream).await
    }

    async fn execute_command(command: &str, command_args: &Vec<String>, shell: bool) -> String {
        let mut subprocess;
        if shell {
            subprocess = tokio::process::Command::new("sh"); // TODO not really portable...
            subprocess.arg("-c");
            let command = format!("{command} {}", command_args.join(" "));
            subprocess.arg(command);
        } else {
            subprocess = tokio::process::Command::new(command);
            subprocess.args(command_args);
        };
        let subprocess = subprocess
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();
        let subprocess = match subprocess {
            Ok(x) => x,
            Err(err) => return err.to_string(),
        };

        let subprocess_result = subprocess.wait_with_output().await;
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
        match subprocess_out {
            Ok(x) => x,
            Err(err) => err.to_string(),
        }
    }
}
