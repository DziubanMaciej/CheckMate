use std::time::Duration;

use crate::action::{Action, WatchCommandData, WatchMode};
use check_mate_common::{
    constants::*, fetch_arg, fetch_arg_and_parse, fetch_arg_bool, fetch_arg_string,
    format_args_list, format_text, CommandLineError,
};

#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
    pub server_port: u16,
    pub client_name: Option<String>,
    pub server_connection_backoff: Duration,
    pub server_connection_attempts: u32,
}

impl Config {
    fn parse_action<T>(args: &mut T) -> Result<Action, CommandLineError>
    where
        T: Iterator<Item = String>,
    {
        let action = fetch_arg(
            args,
            CommandLineError::NoValueSpecified("action".to_owned(), "binary name".to_owned()),
        )?;
        let action = match action.as_ref() {
            "read" => Action::ReadMessages(DEFAULT_INCLUDE_NAMES),
            "watch" => {
                let command = fetch_arg(
                    args,
                    CommandLineError::NoValueSpecified("command to run".to_owned(), action),
                )?;
                let mut command_args = Vec::new();
                for arg in args.by_ref() {
                    if arg != "--" {
                        command_args.push(arg);
                    } else {
                        break; // end of watch args
                    }
                }
                Action::WatchCommand(WatchCommandData::new(command, command_args))
            }
            "refresh" => {
                let name = fetch_arg(
                    args,
                    CommandLineError::NoValueSpecified("client name".to_owned(), action),
                )?;
                Action::RefreshClientByName(name)
            }
            "refresh_all" => Action::RefreshAllClients,
            "list" => Action::ListClients,
            "abort" => Action::Abort,
            "help" | "-h" => Action::Help,
            "version" | "-v" => Action::Version,
            _ => return Err(CommandLineError::InvalidValue("action".into(), action)),
        };
        Ok(action)
    }

    fn parse_extra_args<T: Iterator<Item = String>>(
        &mut self,
        args: &mut T,
    ) -> Result<(), CommandLineError> {
        while let Some(arg) = args.next() {
            match arg.as_ref() {
                "-p" => {
                    self.server_port = fetch_arg_and_parse(
                        args,
                        || CommandLineError::NoValueSpecified("port".into(), arg.clone()),
                        |value| CommandLineError::InvalidValue("port".into(), value.into()),
                    )?;
                }
                "-n" => {
                    self.client_name = Some(fetch_arg_string(
                        args,
                        || CommandLineError::NoValueSpecified("client name".into(), arg.clone()),
                        || CommandLineError::NoValueSpecified("client name".into(), arg.clone()),
                    )?);
                }
                "-i" => {
                    let include_names = match self.action {
                        Action::ReadMessages(ref mut include_names) => include_names,
                        _ => return Err(CommandLineError::InvalidArgument(arg)),
                    };
                    *include_names = fetch_arg_bool(
                        args,
                        || {
                            CommandLineError::NoValueSpecified(
                                "a boolean value".into(),
                                arg.clone(),
                            )
                        },
                        |value| {
                            CommandLineError::InvalidValue("include names".into(), value.into())
                        },
                    )?;
                }
                "-w" => {
                    let data = match self.action {
                        Action::WatchCommand(ref mut data) => data,
                        _ => return Err(CommandLineError::InvalidArgument(arg)),
                    };
                    let interval: u64 = fetch_arg_and_parse(
                        args,
                        || CommandLineError::NoValueSpecified("watch interval".into(), arg.clone()),
                        |value| {
                            CommandLineError::InvalidValue("watch interval".into(), value.into())
                        },
                    )?;
                    data.interval = Duration::from_millis(interval);
                }
                "-d" => {
                    let data = match self.action {
                        Action::WatchCommand(ref mut data) => data,
                        _ => return Err(CommandLineError::InvalidArgument(arg)),
                    };
                    let delay: u64 = fetch_arg_and_parse(
                        args,
                        || CommandLineError::NoValueSpecified("initial delay".into(), arg.clone()),
                        |value| {
                            CommandLineError::InvalidValue("initial delay".into(), value.into())
                        },
                    )?;
                    data.delay = Duration::from_millis(delay);
                }
                "-c" => {
                    let duration: u64 = fetch_arg_and_parse(
                        args,
                        || {
                            CommandLineError::NoValueSpecified(
                                "connection backoff".into(),
                                arg.clone(),
                            )
                        },
                        |value| {
                            CommandLineError::InvalidValue(
                                "connection backoff".into(),
                                value.into(),
                            )
                        },
                    )?;
                    self.server_connection_backoff = Duration::from_millis(duration);
                }
                "-r" => {
                    self.server_connection_attempts = fetch_arg_and_parse(
                        args,
                        || {
                            CommandLineError::NoValueSpecified(
                                "number of connection attempts".into(),
                                arg.clone(),
                            )
                        },
                        |value| {
                            CommandLineError::InvalidValue(
                                "number of connection attempts".into(),
                                value.into(),
                            )
                        },
                    )?;
                }
                "-m" => {
                    let data = match self.action {
                        Action::WatchCommand(ref mut data) => data,
                        _ => return Err(CommandLineError::InvalidArgument(arg)),
                    };
                    data.mode = fetch_arg_and_parse(
                        args,
                        || CommandLineError::NoValueSpecified("watch mode".into(), arg.clone()),
                        |value| CommandLineError::InvalidValue("watch mode".into(), value.into()),
                    )?;
                }
                "-s" => {
                    let shell = match self.action {
                        Action::WatchCommand(ref mut data) => &mut data.shell,
                        _ => return Err(CommandLineError::InvalidArgument(arg)),
                    };
                    *shell = fetch_arg_bool(
                        args,
                        || {
                            CommandLineError::NoValueSpecified(
                                "a boolean value".into(),
                                arg.clone(),
                            )
                        },
                        |value| CommandLineError::InvalidValue("shell".into(), value.into()),
                    )?;
                }
                _ => return Err(CommandLineError::InvalidArgument(arg)),
            }
        }
        Ok(())
    }

    pub fn parse<T>(mut args: T) -> Result<Config, CommandLineError>
    where
        T: Iterator<Item = String>,
    {
        let mut config = Config {
            action: Config::parse_action(&mut args)?,
            ..Default::default()
        };
        if !matches!(config.action, Action::Help | Action::Version) {
            // Help action doesn't need any more arguments, just print help and exit
            config.parse_extra_args(&mut args)?;
        }
        Ok(config)
    }

    pub fn print_help() {
        let intro = "Usage: check_mate_client <action> [<args>]";
        println!("{}\n", format_text(intro, HELP_MESSAGE_MAX_LINE_WIDTH));

        let action_intro = "Available actions:";
        println!("{}", format_text(action_intro, HELP_MESSAGE_MAX_LINE_WIDTH));

        let actions = [
            ("read", "Query error statuses from server".to_owned()),
            ("watch <command>", "Periodically execute <command> and send its output as status to server.".to_owned()),
            ("refresh <name>", "Instruct the server to notify a client with a name equal to <name> to rerun its command immediately and update the status.".to_owned()),
            ("refresh_all", "Instruct the server to notify all its clients to rerun their commands immediately and update the statuses.".to_owned()),
            ("list", "List all existing clients connected to the server.".to_owned()),
            ("abort", "Instruct the server to end execution.".to_owned()),
            ("help", "Print this message.".to_owned()),
            ("version", "Print version.".to_owned()),
        ];
        println!(
            "{}\n",
            format_args_list(
                &actions,
                HELP_MESSAGE_BASIC_INDENT_WIDTH,
                HELP_MESSAGE_MAX_LINE_WIDTH
            )
        );

        let arguments_intro = "
            There is a number of additional arguments that can be passed to the client. Some of them are
            action-specific and will not work with other actions. Arguments are specified after
            action. For watch action, an additional '--' separator is neccessary to divide the command
            arguments and CheckMate arguments. Available arguments:";
        println!(
            "{}",
            format_text(arguments_intro, HELP_MESSAGE_MAX_LINE_WIDTH)
        );

        let watch_modes_descriptions = [
            " - OneLineError. Empty stdout means success. Non-empty stdout means error. The first non-empty line is an error message, the rest is ignored.",
            " - MultiLineError. Empty stdout means success. Non-empty stdout means error. All non-empty lines are error message. Empty lines are ignored.",
            " - ExitCode. Exit code equal to 0 means success. Exit code other than 0 means error. Error message is composed automatically to contain the exit code. The first non-empty in stdout line is an error message, the rest is ignored.",
            " - OneLineErrorExitCode. Exit code equal to 0 means success. Exit code other than 0 means error. If there are no non-empty lines, error message is composed as for ExitCode."
        ];
        let arguments = [
            ("-p <number>", format!("Set TCP port of the server to connect to. Default is {DEFAULT_PORT}.")),
            ("-n <string>", "Set name of this client. Name is optional, but makes it easier to identify clients and allows to refresh them by name.".to_owned()),
            ("-i <boolean>", format!("Only valid with read action. Set whether client names should be printed along with their statuses. Default is {DEFAULT_INCLUDE_NAMES}.", )),
            ("-w <milliseconds>", format!("Only valid with watch action. Set interval in milliseconds between invocation of the watched command. Default is {}ms.", DEFAULT_WATCH_INTERVAL.as_millis())),
            ("-d <milliseconds>", format!("Only valid with watch action. Set delay in milliseconds before the watched command is called for the first time. Default is {}ms.", DEFAULT_WATCH_DELAY.as_millis())),
            ("-m <boolean>", format!("Only valid with watch action. Set watch mode, which represents how errors are detected and reported. Supported modes are listed below. Default is {}.\n{}", WatchMode::default(), watch_modes_descriptions.join("\n"))),
            ("-s <boolean>", format!("Only valid with watch action. Set whether the watched command should be invoked through default OS shell. Default is {DEFAULT_SHELL}.")),
            ("-c <milliseconds>", format!("Set backoff time to wait before retrying after unsuccessful connection to the server. Default is {}ms.", DEFAULT_CONNECTION_BACKOFF.as_millis())),
            ("-r <number>", format!("Set the maximum number of attempts to connect to the server. The value of 0 means infinite attempts. Default is {DEFAULT_MAXIMUM_SERVER_CONNECTION_ATTEMPTS}.")),
        ];
        println!(
            "{}",
            format_args_list(
                &arguments,
                HELP_MESSAGE_BASIC_INDENT_WIDTH,
                HELP_MESSAGE_MAX_LINE_WIDTH
            )
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            action: Action::Abort,
            server_port: DEFAULT_PORT,
            client_name: None,
            server_connection_backoff: DEFAULT_CONNECTION_BACKOFF,
            server_connection_attempts: DEFAULT_MAXIMUM_SERVER_CONNECTION_ATTEMPTS,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_owned_string_iter(string_slices: &[&str]) -> <Vec<String> as IntoIterator>::IntoIter {
        let vector: Vec<String> = string_slices
            .iter()
            .map(|string_slice| string_slice.to_string())
            .collect();
        vector.into_iter()
    }

    #[test]
    fn read_action_is_parsed() {
        let args = ["read"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::ReadMessages(false);
        assert_eq!(config, expected);
    }

    #[test]
    fn read_action_with_include_names_argument_is_parsed() {
        fn run(include_names: &str, include_names_bool: bool) {
            let args = ["read", "-i", include_names];
            let config = Config::parse(to_owned_string_iter(&args));
            let config = config.expect("Parsing should succeed");

            let mut expected = Config::default();
            expected.action = Action::ReadMessages(include_names_bool);
            assert_eq!(config, expected);
        }
        run("0", false);
        run("false", false);
        run("1", true);
        run("true", true);
    }

    #[test]
    fn read_action_with_invalid_include_names_argument_should_fail() {
        fn run(include_names: &str) {
            let args = ["read", "-i", include_names];
            let config = Config::parse(to_owned_string_iter(&args));
            let err = config.expect_err("Parsing should fail");
            let expected =
                CommandLineError::InvalidValue("include names".into(), include_names.into());
            assert_eq!(err, expected);
        }
        run("aa");
        run("");
        run("1.");
        run("1 .");
    }

    #[test]
    fn watch_action_is_parsed() {
        let args = ["watch", "whoami"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action =
            Action::WatchCommand(WatchCommandData::new("whoami".to_string(), Vec::new()));
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_action_with_command_with_no_args_is_parsed() {
        let args = ["watch", "whoami"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action =
            Action::WatchCommand(WatchCommandData::new("whoami".to_string(), Vec::new()));
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_action_with_args_is_parsed() {
        let args = ["watch", "whoami", "hello", "world"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::WatchCommand(WatchCommandData::new(
            "whoami".to_string(),
            vec!["hello".to_string(), "world".to_string()],
        ));
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_action_with_dash_args_is_parsed() {
        let args = ["watch", "whoami", "-p", "101", "--", "-p", "100"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::WatchCommand(WatchCommandData::new(
            "whoami".to_string(),
            vec!["-p".to_string(), "101".to_string()],
        ));
        expected.server_port = 100;
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_action_with_mode_argument_is_parsed() {
        fn run(value: &str, mode: WatchMode) {
            let args = ["watch", "echo", "a", "--", "-m", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let config = config.expect("Parsing should succeed");

            let mut watch_command_data =
                WatchCommandData::new("echo".to_string(), vec!["a".to_string()]);
            watch_command_data.mode = mode;
            let mut expected = Config::default();
            expected.action = Action::WatchCommand(watch_command_data);
            assert_eq!(config, expected);
        }
        run("OneLineError", WatchMode::OneLineError);
        run("OneLineErROR", WatchMode::OneLineError);
        run("MultiLineError", WatchMode::MultiLineError);
        run("MultiLineErROR", WatchMode::MultiLineError);
        run("ExitCode", WatchMode::ExitCode);
        run("ExitCODE", WatchMode::ExitCode);
        run("OneLineErrorExitCode", WatchMode::OneLineErrorExitCode);
        run("OneLineErrorExitCODE", WatchMode::OneLineErrorExitCode);
    }

    #[test]
    fn watch_action_with_invalid_mode_argument_should_fail() {
        fn run(value: &str) {
            let args = ["watch", "echo", "a", "--", "-m", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let err = config.expect_err("Parsing should fail");
            let expected = CommandLineError::InvalidValue("watch mode".into(), value.into());
            assert_eq!(err, expected);
        }
        run("OneLineErro");
        run("");
        run("OneLineErrorrrrrrr");
        run("1");
        run("0");
        run(".");
        run("*");
    }

    #[test]
    fn watch_action_with_shell_argument_is_parsed() {
        fn run(value: &str, value_bool: bool) {
            let args = ["watch", "echo", "a", "--", "-s", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let config = config.expect("Parsing should succeed");

            let mut watch_command_data =
                WatchCommandData::new("echo".to_string(), vec!["a".to_string()]);
            watch_command_data.shell = value_bool;
            let mut expected = Config::default();
            expected.action = Action::WatchCommand(watch_command_data);
            assert_eq!(config, expected);
        }
        run("0", false);
        run("false", false);
        run("1", true);
        run("true", true);
    }

    #[test]
    fn watch_action_with_invalid_shell_argument_should_fail() {
        fn run(value: &str) {
            let args = ["watch", "echo", "a", "--", "-s", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let err = config.expect_err("Parsing should fail");
            let expected = CommandLineError::InvalidValue("shell".into(), value.into());
            assert_eq!(err, expected);
        }
        run("aa");
        run("");
        run("1.");
        run("1 .");
    }

    #[test]
    fn refresh_action_is_parsed() {
        let args = ["refresh", "client12"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::RefreshClientByName("client12".to_string());
        assert_eq!(config, expected);
    }

    #[test]
    fn refresh_all_action_is_parsed() {
        let args = ["refresh_all"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::RefreshAllClients;
        assert_eq!(config, expected);
    }

    #[test]
    fn list_clients_action_is_parsed() {
        let args = ["list"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::ListClients;
        assert_eq!(config, expected);
    }

    #[test]
    fn abort_action_is_parsed() {
        let args = ["abort"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::Abort;
        assert_eq!(config, expected);
    }

    #[test]
    fn help_action_is_parsed() {
        fn run(args: &[&str]) {
            let config = Config::parse(to_owned_string_iter(&args));
            let config = config.expect("Parsing should succeed");

            let mut expected = Config::default();
            expected.action = Action::Help;
            assert_eq!(config, expected);
        }

        run(&["help"]);
        run(&["help", "-p", "200"]);
        run(&["-h"]);
        run(&["-h", "-n", "client"]);
    }

    #[test]
    fn version_action_is_parsed() {
        fn run(args: &[&str]) {
            let config = Config::parse(to_owned_string_iter(&args));
            let config = config.expect("Parsing should succeed");

            let mut expected = Config::default();
            expected.action = Action::Version;
            assert_eq!(config, expected);
        }

        run(&["version"]);
        run(&["version", "-p", "200"]);
        run(&["-v"]);
        run(&["-v", "-n", "client"]);
    }

    #[test]
    fn custom_port_number_is_parsed() {
        let args = ["refresh", "client12", "-p", "10"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::RefreshClientByName("client12".to_string());
        expected.server_port = 10;
        assert_eq!(config, expected);
    }

    #[test]
    fn custom_connection_attempts_option_is_parsed() {
        fn run(value_string: &str, value: u32) {
            let args = ["refresh", "client12", "-r", value_string];
            let config = Config::parse(to_owned_string_iter(&args));
            let config = config.expect("Parsing should succeed");

            let mut expected = Config::default();
            expected.action = Action::RefreshClientByName("client12".to_string());
            expected.server_connection_attempts = value;
            assert_eq!(config, expected);
        }

        run("0", 0);
        run("1", 1);
        run("100", 100);
    }

    #[test]
    fn custom_client_name_is_parsed() {
        let args = ["refresh", "client12", "-n", "client11"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::RefreshClientByName("client12".to_string());
        expected.client_name = Some("client11".to_string());
        assert_eq!(config, expected);
    }

    #[test]
    fn server_connection_backoff_is_parsed() {
        let args = ["refresh", "client12", "-c", "400"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::RefreshClientByName("client12".to_string());
        expected.server_connection_backoff = Duration::from_millis(400);
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_interval_is_parsed() {
        let args = ["watch", "echo", "--", "-w", "123"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        let mut watch_command_data = WatchCommandData::new("echo".into(), Vec::new());
        watch_command_data.interval = Duration::from_millis(123);
        expected.action = Action::WatchCommand(watch_command_data);
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_initial_delay_is_parsed() {
        let args = ["watch", "echo", "--", "-d", "123"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        let mut watch_command_data = WatchCommandData::new("echo".into(), Vec::new());
        watch_command_data.delay = Duration::from_millis(123);
        expected.action = Action::WatchCommand(watch_command_data);
        assert_eq!(config, expected);
    }

    #[test]
    fn multiple_custom_args_are_parsed() {
        let args = [
            "refresh", "client12", "-n", "client11", "-p", "120", "-c", "400",
        ];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::RefreshClientByName("client12".to_string());
        expected.server_port = 120;
        expected.client_name = Some("client11".to_string());
        expected.server_connection_backoff = Duration::from_millis(400);
        assert_eq!(config, expected);
    }

    #[test]
    fn no_action_error_is_returned() {
        let args = [];
        let config = Config::parse(args.into_iter());
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("action".to_owned(), "binary name".to_owned());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_watch_command_error_is_returned() {
        let args = ["watch"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("command to run".to_owned(), "watch".to_owned());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_client_name_error_to_refresh_is_returned() {
        let args = ["refresh"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("client name".to_owned(), "refresh".to_owned());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_port_error_is_returned() {
        let args = ["read", "-p"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected = CommandLineError::NoValueSpecified("port".to_string(), "-p".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_connection_attempts_number_error_is_returned() {
        let args = ["read", "-r"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected = CommandLineError::NoValueSpecified(
            "number of connection attempts".to_string(),
            "-r".to_string(),
        );
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_server_connection_backoff_error_is_returned() {
        let args = ["read", "-c"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("connection backoff".to_string(), "-c".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_watch_interval_error_is_returned() {
        let args = ["watch", "echo", "--", "-w"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("watch interval".to_string(), "-w".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_initial_delay_error_is_returned() {
        let args = ["watch", "echo", "--", "-d"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("initial delay".to_string(), "-d".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn invalid_action_error_is_returned() {
        let args = ["jump"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected = CommandLineError::InvalidValue("action".to_string(), "jump".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_client_name_error_is_returned() {
        let args = ["read", "-n"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("client name".to_string(), "-n".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn empty_client_name_error_is_returned() {
        let args = ["read", "-n", ""];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected =
            CommandLineError::NoValueSpecified("client name".to_string(), "-n".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn invalid_port_error_is_returned() {
        {
            let args = ["read", "-p", "-1"];
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected = CommandLineError::InvalidValue("port".to_string(), "-1".to_string());
            assert_eq!(parse_error, expected);
        }
        {
            let args = ["read", "-p", "s"];
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected = CommandLineError::InvalidValue("port".to_string(), "s".to_string());
            assert_eq!(parse_error, expected);
        }
        {
            let args = ["read", "-p", "2000d"];
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected = CommandLineError::InvalidValue("port".to_string(), "2000d".to_string());
            assert_eq!(parse_error, expected);
        }
    }

    #[test]
    fn invalid_number_of_connection_attemps_error_is_returned() {
        fn run(value: &str) {
            let args = ["read", "-r", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected = CommandLineError::InvalidValue(
                "number of connection attempts".to_string(),
                value.to_string(),
            );
            assert_eq!(parse_error, expected);
        }
        run("");
        run("-1");
        run("ss");
        run("200d");
    }

    #[test]
    fn invalid_server_connection_backoff_error_is_returned() {
        fn run(value: &str) {
            let args = ["read", "-c", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected =
                CommandLineError::InvalidValue("connection backoff".to_string(), value.to_string());
            assert_eq!(parse_error, expected);
        }
        run(" ");
        run("");
        run("40f");
        run("40 f");
        run("abc");
    }

    #[test]
    fn invalid_watch_interval_error_is_returned() {
        fn run(value: &str) {
            let args = ["watch", "echo", "--", "-w", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected =
                CommandLineError::InvalidValue("watch interval".to_string(), value.to_string());
            assert_eq!(parse_error, expected);
        }
        run(" ");
        run("");
        run("40f");
        run("40 f");
        run("abc");
    }

    #[test]
    fn invalid_initial_delay_error_is_returned() {
        fn run(value: &str) {
            let args = ["watch", "echo", "--", "-d", value];
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected =
                CommandLineError::InvalidValue("initial delay".to_string(), value.to_string());
            assert_eq!(parse_error, expected);
        }
        run(" ");
        run("");
        run("40f");
        run("40 f");
        run("abc");
    }

    #[test]
    fn invalid_argument_error_is_returned() {
        let args = ["read", "-k"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected = CommandLineError::InvalidArgument("-k".to_string());
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn command_specific_extra_args_return_error_when_used_with_wrong_command() {
        let command_specific_args = [("-i", "1"), ("-w", "123")];

        for (arg, value) in command_specific_args {
            let args = ["abort", arg, value]; // abort is a command with no command-specific args, so we can use it here
            let config = Config::parse(to_owned_string_iter(&args));
            let parse_error = config.expect_err("Parsing should not succeed");

            let expected = CommandLineError::InvalidArgument(arg.to_string());
            assert_eq!(parse_error, expected);
        }
    }
}
