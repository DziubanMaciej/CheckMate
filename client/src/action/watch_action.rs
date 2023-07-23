use super::definition::Action;
use check_mate_common::constants::*;
use check_mate_common::{CommunicationError, ServerCommand};
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncWrite};

#[derive(PartialEq, Debug)]
pub enum WatchMode {
    /// Empty stdout means success.
    /// Non-empty stdout means error. The first non-empty line is an error message, the rest is ignored.
    OneLineError,

    /// Empty stdout means success.
    /// Non-empty stdout means error. All non-empty lines are error message. Empty lines are ignored.
    MultiLineError,

    /// Exit code equal to 0 means success.
    /// Exit code other than 0 means error. Error message is composed automatically to contain the exit code.
    ExitCode,

    /// Exit code equal to 0 means success.
    /// Exit code other than 0 means error. The first non-empty in stdout line is an error message, the rest is ignored.
    /// If there are no non-empty lines, error message is composed as for ExitCode.
    OneLineErrorExitCode,
}

impl std::str::FromStr for WatchMode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "onelineerror" => Ok(Self::OneLineError),
            "multilineerror" => Ok(Self::MultiLineError),
            "exitcode" => Ok(Self::ExitCode),
            "onelineerrorexitcode" => Ok(Self::OneLineErrorExitCode),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for WatchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let display_str = match self {
            WatchMode::OneLineError => "OneLineError",
            WatchMode::MultiLineError => "MultiLineError",
            WatchMode::ExitCode => "ExitCode",
            WatchMode::OneLineErrorExitCode => "OneLineErrorExitCode",
        };
        write!(f, "{}", display_str)
    }
}

impl Default for WatchMode {
    fn default() -> Self {
        return WatchMode::OneLineError;
    }
}

#[derive(PartialEq, Debug)]
pub struct WatchCommandData {
    pub command: String,
    pub command_args: Vec<String>,
    pub mode: WatchMode,
    pub interval: Duration,
    pub shell: bool,
    pub delay: Duration,
}

impl WatchCommandData {
    pub fn new(command: String, command_args: Vec<String>) -> Self {
        Self {
            command,
            command_args,
            mode: WatchMode::default(),
            interval: DEFAULT_WATCH_INTERVAL,
            shell: DEFAULT_SHELL,
            delay: DEFAULT_WATCH_DELAY,
        }
    }
}

#[derive(Clone)]
struct ExecuteCommandOutput {
    executed: bool,
    status: Option<i32>,
    text: String,
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
            let server_command = match Action::process_command_output(command_output, &data.mode) {
                Ok(_) => ServerCommand::SetStatusOk,
                Err(x) => ServerCommand::SetStatusError(x),
            };

            // Send status to the server
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

    async fn execute_command(
        command: &str,
        command_args: &Vec<String>,
        shell: bool,
    ) -> ExecuteCommandOutput {
        // Try to spawn subprocess
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

        // Handle failure to spawn the subprocess
        let subprocess = match subprocess {
            Ok(x) => x,
            Err(err) => {
                let text = match err.kind() {
                    std::io::ErrorKind::NotFound => format!("Executable \"{command}\" not found"),
                    _ => err.to_string(),
                };
                return ExecuteCommandOutput {
                    executed: false,
                    status: None,
                    text,
                };
            }
        };

        // Wait for command to end and handle failure of waiting
        let subprocess_result = subprocess.wait_with_output().await;
        let subprocess_result = match subprocess_result {
            Ok(x) => x,
            Err(err) => {
                return ExecuteCommandOutput {
                    executed: false,
                    status: None,
                    text: err.to_string(),
                }
            }
        };

        // The command has completed. Return information about it
        ExecuteCommandOutput {
            executed: true,
            status: subprocess_result.status.code(),
            text: String::from_utf8(subprocess_result.stdout)
                .unwrap_or("Could not parse stdout".to_owned()),
        }
    }

    fn process_command_output(
        output: ExecuteCommandOutput,
        watch_mode: &WatchMode,
    ) -> Result<(), String> {
        // Handle case when the command wasn't even executed
        if !output.executed {
            return Err(format!("Command was not executed. {}", output.text));
        }

        // Helper closures
        let process_one_line_error = || {
            let first_line = output
                .text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .take(1)
                .next();
            match first_line {
                Some(x) => {
                    let first_line = x.trim().to_owned();
                    Err(first_line)
                }
                None => Ok(()),
            }
        };
        let process_multi_line_error = || {
            let command_output = output
                .text
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::trim)
                .collect::<Vec<_>>();
            if command_output.is_empty() {
                Ok(())
            } else {
                Err(command_output.join("\n"))
            }
        };
        let process_exit_code = |code: i32| {
            if code == 0 {
                Ok(())
            } else {
                Err(format!("Exit code was {code}"))
            }
        };

        // Main match statement. Each WatchMode has to be handled differently.
        match watch_mode {
            WatchMode::OneLineError => process_one_line_error(),
            WatchMode::MultiLineError => process_multi_line_error(),
            WatchMode::ExitCode => match output.status {
                None => Err("Exit code is not available".to_owned()),
                Some(x) => process_exit_code(x),
            },
            WatchMode::OneLineErrorExitCode => match output.status {
                None => Err("Exit code is not available".to_owned()),
                Some(x) if x != 0 => process_one_line_error(),
                Some(x) => process_exit_code(x),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_all_watch_modes() -> impl Iterator<Item = WatchMode> {
        [
            WatchMode::OneLineError,
            WatchMode::MultiLineError,
            WatchMode::ExitCode,
            WatchMode::OneLineErrorExitCode,
        ]
        .into_iter()
    }

    #[test]
    fn given_command_not_executed_when_processing_command_ouptput_then_return_error() {
        let command_output = ExecuteCommandOutput {
            executed: false,
            status: None,
            text: "Hello".to_owned(),
        };
        let expected_result = Err("Command was not executed. Hello".to_owned());
        for watch_mode in get_all_watch_modes() {
            let actual_result = Action::process_command_output(command_output.clone(), &watch_mode);
            assert_eq!(expected_result, actual_result);
        }
    }

    #[test]
    fn given_one_line_error_mode_when_processing_command_output_then_return_correct_result() {
        fn run(command_stdout: &str, expected_result: Result<(), String>) {
            // Exit status should not matter for this mode, so we check multiple options and the
            // result should be the same for all of them.
            let statuses = [None, Some(0), Some(1)];
            for status in statuses {
                let command_output = ExecuteCommandOutput {
                    executed: true,
                    status,
                    text: command_stdout.to_owned(),
                };

                let watch_mode = WatchMode::OneLineError;
                let actual_result =
                    Action::process_command_output(command_output.clone(), &watch_mode);
                assert_eq!(expected_result, actual_result);
            }
        }

        run("", Ok(()));
        run("   ", Ok(()));
        run("   \n  \n", Ok(()));
        run("hello", Err("hello".to_owned()));
        run(" hello", Err("hello".to_owned()));
        run("\thello", Err("hello".to_owned()));
        run("\nhello", Err("hello".to_owned()));
        run("\n hello", Err("hello".to_owned()));
        run("\n\n   \n   hello\nworld\n   hi", Err("hello".to_owned()));
    }

    #[test]
    fn given_multi_line_error_mode_when_processing_command_output_then_return_correct_result() {
        fn run(command_stdout: &str, expected_result: Result<(), String>) {
            // Exit status should not matter for this mode, so we check multiple options and the
            // result should be the same for all of them.
            let statuses = [None, Some(0), Some(1)];
            for status in statuses {
                let command_output = ExecuteCommandOutput {
                    executed: true,
                    status,
                    text: command_stdout.to_owned(),
                };

                let watch_mode = WatchMode::MultiLineError;
                let actual_result =
                    Action::process_command_output(command_output.clone(), &watch_mode);
                assert_eq!(expected_result, actual_result);
            }
        }

        run("", Ok(()));
        run("   ", Ok(()));
        run("   \n  \n", Ok(()));
        run("hello", Err("hello".to_owned()));
        run("\nhello", Err("hello".to_owned()));
        run("\n hello", Err("hello".to_owned()));
        run(
            "hello\nworld\nhi\ngood morning",
            Err("hello\nworld\nhi\ngood morning".to_owned()),
        );
        run(
            "\n\n   \n   hello\nworld\n\n\n  \n\t   hi",
            Err("hello\nworld\nhi".to_owned()),
        );
    }

    #[test]
    fn given_exit_code_mode_when_processing_command_output_then_return_correct_error() {
        fn run(status: Option<i32>, expected_result: Result<(), String>) {
            // Stdout contents should not matter for this mode, so we check multiple strings and the
            // result should be the same for all of them.
            let texts = ["", "hello", "hello\nworld"];
            for text in texts {
                let command_output = ExecuteCommandOutput {
                    executed: true,
                    status,
                    text: text.to_owned(),
                };

                let watch_mode = WatchMode::ExitCode;
                let actual_result =
                    Action::process_command_output(command_output.clone(), &watch_mode);
                assert_eq!(expected_result, actual_result);
            }
        }

        run(None, Err("Exit code is not available".to_owned()));
        run(Some(0), Ok(()));
        run(Some(1), Err("Exit code was 1".to_owned()));
        run(Some(-1), Err("Exit code was -1".to_owned()));
        run(Some(127), Err("Exit code was 127".to_owned()));
    }

    #[test]
    fn given_one_line_error_exit_code_mode_when_processing_command_output_then_return_correct_result(
    ) {
        fn run(status: Option<i32>, command_stdout: &str, expected_result: Result<(), String>) {
            let command_output = ExecuteCommandOutput {
                executed: true,
                status,
                text: command_stdout.to_owned(),
            };

            let watch_mode = WatchMode::OneLineErrorExitCode;
            let actual_result = Action::process_command_output(command_output.clone(), &watch_mode);
            assert_eq!(expected_result, actual_result);
        }

        run(None, "hello", Err("Exit code is not available".to_owned()));
        run(Some(0), "", Ok(()));
        run(Some(0), "hello", Ok(()));
        run(Some(10), "hello", Err("hello".to_owned()));
        run(Some(10), "hello\nworld", Err("hello".to_owned()));
    }
}
