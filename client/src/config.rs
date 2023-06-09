use crate::action::Action;
use check_mate_common::{fetch_arg, CommandLineError, DEFAULT_PORT};

#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
    pub server_port: u16,
    pub client_name: Option<String>,
}

impl Config {
    fn parse_action<T: Iterator<Item = String>>(args: &mut T) -> Result<Action, CommandLineError> {
        let action = fetch_arg(
            args,
            CommandLineError::NoValueSpecified("action".to_owned(), "binary name".to_owned()),
        )?;
        let action = match action.as_ref() {
            "read" => Action::ReadMessages,
            "watch" => {
                let command = fetch_arg(
                    args,
                    CommandLineError::NoValueSpecified("command to run".to_owned(), action),
                )?;
                Action::WatchCommand(command, Vec::new())
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
                    let port = fetch_arg(
                        args,
                        CommandLineError::NoValueSpecified("port".into(), arg),
                    )?;
                    let port = match port.parse::<u16>() {
                        Ok(x) => x,
                        Err(_) => return Err(CommandLineError::InvalidValue("port".into(), port)),
                    };
                    self.server_port = port;
                }
                "-n" => {
                    let name = fetch_arg(
                        args,
                        CommandLineError::NoValueSpecified("client name".into(), arg.clone()),
                    )?;
                    if name == "" {
                        return Err(CommandLineError::NoValueSpecified(
                            "client name".into(),
                            arg,
                        ));
                    }
                    self.client_name = Some(name);
                }
                "--args" => {
                    if let Action::WatchCommand(_, ref mut command_args) = self.action {
                        loop {
                            match args.next() {
                                Some(x) => command_args.push(x),
                                None => break,
                            };
                        }
                    } else {
                        return Err(CommandLineError::InvalidArgument(arg));
                    }
                }
                _ => return Err(CommandLineError::InvalidArgument(arg)),
            }
        }
        Ok(())
    }

    pub fn parse<T: Iterator<Item = String>>(mut args: T) -> Result<Config, CommandLineError> {
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
            action: Action::ReadMessages,
            server_port: DEFAULT_PORT,
            client_name: None,
        };
        assert_eq!(config, expected);
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
    fn watch_action_with_empty_args_is_parsed() {
        let args = ["watch", "whoami", "--args"];
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
        let args = ["watch", "whoami", "--args", "hello", "world"];
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
        let args = ["watch", "whoami", "-p", "100", "--args", "-p", "101"];
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
