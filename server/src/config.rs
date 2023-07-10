use check_mate_common::{fetch_arg, fetch_arg_bool, CommandLineError, DEFAULT_PORT};

#[derive(PartialEq, Debug, Clone)]
pub struct Config {
    pub server_port: u16,
    pub log_every_status: bool,
    pub help: bool,
}

impl Config {
    fn parse_options<T: Iterator<Item = String>>(
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
                    let port =
                        fetch_arg(args, CommandLineError::NoValueSpecified("port".into(), arg))?;
                    let port = match port.parse::<u16>() {
                        Ok(x) => x,
                        Err(_) => return Err(CommandLineError::InvalidValue("port".into(), port)),
                    };
                    self.server_port = port;
                }
                "-e" => {
                    self.log_every_status = fetch_arg_bool(
                        args,
                        || {
                            CommandLineError::NoValueSpecified(
                                "a boolean value".into(),
                                arg.clone(),
                            )
                        },
                        |value| {
                            CommandLineError::InvalidValue("log every status".into(), value.into())
                        },
                    )?;
                }
                "-h" => {
                    self.help = true;
                }
                _ => return Err(CommandLineError::InvalidArgument(arg)),
            }
        }
        Ok(())
    }

    pub fn parse<T: Iterator<Item = String>>(mut args: T) -> Result<Config, CommandLineError> {
        let mut config = Config::default();
        config.parse_options(&mut args)?;
        Ok(config)
    }

    pub fn print_help() {
        let string = "Usage: check_mate_server [<args>]

Available args:
    - p <port> - Set TCP port for the server.
    - e <boolean> - Set whether the server should log every status received from clients or only when it changes.
    - h - Print this message.
";
        println!("{}", string);
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_port: DEFAULT_PORT,
            log_every_status: false,
            help: false,
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
    fn no_args_returns_default_config() {
        let args = [];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let expected = Config::default();
        assert_eq!(config, expected);
    }

    #[test]
    fn server_port_is_parsed() {
        let args = ["-p", "123"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.server_port = 123;
        assert_eq!(config, expected);
    }

    #[test]
    fn log_every_status_is_parsed() {
        let args = ["-e", "1"];
        let config = Config::parse(to_owned_string_iter(&args));
        let config = config.expect("Parsing should succeed");

        let mut expected = Config::default();
        expected.log_every_status = true;
        assert_eq!(config, expected);
    }
}
