use crate::action::Action;

#[derive(PartialEq, Debug)]
pub struct Config {
    pub action: Action,
    pub server_port: u16,
    pub client_name: Option<String>,
}

#[derive(PartialEq, Debug)]
pub enum CommandLineError {
    NoActionSpecified,
    NoWatchCommandSpecified,
    NoClientNameSpecified,
    NoValueSpecified(String, String),

    InvalidValue(String, String),
    InvalidArgument(String),
}

impl std::fmt::Display for CommandLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::NoActionSpecified => write!(f, "Specify action as a first argument"),
            Self::NoWatchCommandSpecified => write!(f, "Specify command to watch"),
            Self::NoClientNameSpecified => write!(f, "Specify client name to refresh"),
            Self::NoValueSpecified(name, option) => {
                write!(f, "Specify a {} value after {}", name, option)
            }
            Self::InvalidValue(name, value) => {
                write!(f, "Invalid {} value specified: {}", name, value)
            }
            Self::InvalidArgument(arg) => write!(f, "Invalid argument specified: {}", arg),
        }?;
        Ok(())
    }
}

impl Config {
    pub(crate) const DEFAULT_PORT: u16 = 10005; // TODO move to common

    fn fetch_arg<T: Iterator<Item = String>>(
        args: &mut T,
        on_error: CommandLineError,
    ) -> Result<String, CommandLineError> {
        match args.next() {
            Some(x) => Ok(x),
            None => return Err(on_error),
        }
    }

    fn parse_action<T: Iterator<Item = String>>(args: &mut T) -> Result<Action, CommandLineError> {
        let action = Config::fetch_arg(args, CommandLineError::NoActionSpecified)?;
        let action = match action.as_ref() {
            "read" => Action::ReadMessages,
            "watch" => {
                let command = Config::fetch_arg(args, CommandLineError::NoWatchCommandSpecified)?;
                Action::WatchCommand(command, Vec::new())
            }
            "refresh" => {
                let name = Config::fetch_arg(args, CommandLineError::NoClientNameSpecified)?;
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
                    let port = Config::fetch_arg(
                        args,
                        CommandLineError::NoValueSpecified("port".into(), "-p".into()),
                    )?;
                    let port = match port.parse::<u16>() {
                        Ok(x) => x,
                        Err(_) => return Err(CommandLineError::InvalidValue("port".into(), port)),
                    };
                    self.server_port = port;
                }
                "-n" => {
                    let name = Config::fetch_arg(
                        args,
                        CommandLineError::NoValueSpecified("client name".into(), "-n".into()),
                    )?;
                    if name == "" {
                        return Err(CommandLineError::NoValueSpecified(
                            "client name".into(),
                            "-n".into(),
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
            server_port: Config::DEFAULT_PORT,
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
            server_port: Config::DEFAULT_PORT,
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
            server_port: Config::DEFAULT_PORT,
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
            server_port: Config::DEFAULT_PORT,
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
            server_port: Config::DEFAULT_PORT,
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
            server_port: Config::DEFAULT_PORT,
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
            server_port: Config::DEFAULT_PORT,
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
            server_port: Config::DEFAULT_PORT,
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

        let expected = CommandLineError::NoActionSpecified;
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_watch_command_error_is_returned() {
        let args = ["watch"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected = CommandLineError::NoWatchCommandSpecified;
        assert_eq!(parse_error, expected);
    }

    #[test]
    fn no_client_name_error_to_refresh_is_returned() {
        let args = ["refresh"];
        let config = Config::parse(to_owned_string_iter(&args));
        let parse_error = config.expect_err("Parsing should not succeed");

        let expected = CommandLineError::NoClientNameSpecified;
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
