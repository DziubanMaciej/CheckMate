use std::time::Duration;

use crate::action::{Action, WatchCommandData};
use check_mate_common::{
    fetch_arg, fetch_arg_and_parse, fetch_arg_bool, fetch_arg_string, CommandLineError,
    DEFAULT_CONNECTION_BACKOFF, DEFAULT_PORT,
};

#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
    pub server_port: u16,
    pub client_name: Option<String>,
    pub server_connection_backoff: Duration,
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
            "read" => Action::ReadMessages(false),
            "watch" => {
                let command = fetch_arg(
                    args,
                    CommandLineError::NoValueSpecified("command to run".to_owned(), action),
                )?;
                let mut command_args = Vec::new();
                loop {
                    if let Some(arg) = args.next() {
                        if arg != "--" {
                            command_args.push(arg);
                        } else {
                            break; // end of watch args
                        }
                    } else {
                        break; // no more args
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
            "abort" => Action::Abort,
            _ => return Err(CommandLineError::InvalidValue("action".into(), action)),
        };
        Ok(action)
    }

    fn parse_extra_args<T: Iterator<Item = String>>(
        &mut self,
        args: &mut T,
    ) -> Result<(), CommandLineError> {
        loop {
            let arg = match args.next() {
                Some(x) => x,
                None => break,
            };

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
                _ => return Err(CommandLineError::InvalidArgument(arg)),
            }
        }
        Ok(())
    }

    pub fn parse<T>(mut args: T) -> Result<Config, CommandLineError>
    where
        T: Iterator<Item = String>,
    {
        let mut config = Config::default();
        config.action = Config::parse_action(&mut args)?;
        config.parse_extra_args(&mut args)?;
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            action: Action::Abort,
            server_port: DEFAULT_PORT,
            client_name: None,
            server_connection_backoff: DEFAULT_CONNECTION_BACKOFF,
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
    fn abort_action_is_parsed() {
        let args = ["abort"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.action = Action::Abort;
        assert_eq!(config, expected);
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
