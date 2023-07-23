use super::definition::Action;
use check_mate_common::constants::*;
use check_mate_common::{CommunicationError, ServerCommand};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncWrite};

#[derive(PartialEq, Debug)]
pub struct WatchCommandData {
    pub command: String,
    pub command_args: Vec<String>,
    pub interval: Duration,
    pub shell: bool,
    pub delay: Duration,
}

impl WatchCommandData {
    pub fn new(command: String, command_args: Vec<String>) -> Self {
        Self {
            command,
            command_args,
            interval: DEFAULT_WATCH_INTERVAL,
            shell: DEFAULT_SHELL,
            delay: DEFAULT_WATCH_DELAY,
        }
    }
}

impl Action {
    pub(crate) async fn watch(
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

        // Run first iteration
        tokio::time::sleep(data.delay).await;
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
            Err(err) => {
                return match err.kind() {
                    std::io::ErrorKind::NotFound => format!("Executable \"{command}\" not found"),
                    _ => err.to_string(),
                };
            }
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
