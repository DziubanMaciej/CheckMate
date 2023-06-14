use crate::action::Action;
use check_mate_common::{
    fetch_arg, fetch_arg_bool, fetch_arg_string, fetch_arg_u16, CommandLineError, DEFAULT_PORT,
};

#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
    pub server_port: u16,
    pub client_name: Option<String>,
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
                Action::WatchCommand(command, command_args)
            }
            "refresh" => {
                let name = fetch_arg(
                    args,
                    CommandLineError::NoValueSpecified("client name".to_owned(), action),
                )?;
                Action::RefreshClientByName(name)
            }
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
                    self.server_port = fetch_arg_u16(
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
                _ => return Err(CommandLineError::InvalidArgument(arg)),
            }
        }
        Ok(())
    }

    pub fn parse<T>(mut args: T) -> Result<Config, CommandLineError>
    where
        T: Iterator<Item = String>,
    {
        let action = Config::parse_action(&mut args)?;
        let mut config = Config {
            action: action,
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        config.parse_extra_args(&mut args)?;
        Ok(config)
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

        let expected = Config {
            action: Action::ReadMessages(false),
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn read_action_with_include_names_argument_is_parsed() {
        fn run(include_names: &str, include_names_bool: bool) {
            let args = ["read", "-i", include_names];
            let config = Config::parse(to_owned_string_iter(&args));
            let config = config.expect("Parsing should succeed");

            let expected = Config {
                action: Action::ReadMessages(include_names_bool),
                server_port: DEFAULT_PORT,
                client_name: None,
            };
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

        let expected = Config {
            action: Action::WatchCommand("whoami".to_string(), Vec::new()),
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_action_with_command_with_no_args_is_parsed() {
        let args = ["watch", "whoami"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::WatchCommand("whoami".to_string(), Vec::new()),
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_action_with_args_is_parsed() {
        let args = ["watch", "whoami", "hello", "world"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::WatchCommand(
                "whoami".to_string(),
                vec!["hello".to_string(), "world".to_string()],
            ),
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn watch_action_with_dash_args_is_parsed() {
        let args = ["watch", "whoami", "-p", "101", "--", "-p", "100"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::WatchCommand(
                "whoami".to_string(),
                vec!["-p".to_string(), "101".to_string()],
            ),
            server_port: 100,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn refresh_action_is_parsed() {
        let args = ["refresh", "client12"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::RefreshClientByName("client12".to_string()),
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn abort_action_is_parsed() {
        let args = ["abort"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::Abort,
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn custom_port_number_is_parsed() {
        let args = ["refresh", "client12", "-p", "10"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::RefreshClientByName("client12".to_string()),
            server_port: 10,
            client_name: None,
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn custom_client_name_is_parsed() {
        let args = ["refresh", "client12", "-n", "client11"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::RefreshClientByName("client12".to_string()),
            server_port: DEFAULT_PORT,
            client_name: Some("client11".to_string()),
        };
        assert_eq!(config, expected);
    }

    #[test]
    fn multiple_custom_args_are_parsed() {
        let args = ["refresh", "client12", "-n", "client11", "-p", "120"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config {
            action: Action::RefreshClientByName("client12".to_string()),
            server_port: 120,
            client_name: Some("client11".to_string()),
        };
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
    fn invalid_argument_error_is_returned() {
        let args = ["read", "-k"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected = CommandLineError::InvalidArgument("-k".to_string());
        assert_eq!(parse_error, expected);
    }
}

/*
pub enum CommandLineError {
    NoActionSpecified,
    NoWatchCommandSpecified,
    NoClientNameSpecified,
    NoValueSpecified(String, String),

    InvalidValue(String, String),
    InvalidArgument(String),
}

 */
