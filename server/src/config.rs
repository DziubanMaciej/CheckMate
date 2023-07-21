use check_mate_common::{
    constants::*, fetch_arg, fetch_arg_bool, format_args_list, format_text, CommandLineError,
};

#[derive(PartialEq, Debug, Clone)]
pub struct Config {
    pub server_port: u16,
    pub log_every_status: bool,
    pub help: bool,
    pub version: bool,
}

impl Config {
    fn parse_options<T: Iterator<Item = String>>(
        &mut self,
        args: &mut T,
    ) -> Result<(), CommandLineError> {
        while let Some(arg) = args.next() {
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
                "-v" => {
                    self.version = true;
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
        let intro = "Usage: check_mate_server [<args>]";
        println!("{}\n", format_text(intro, HELP_MESSAGE_MAX_LINE_WIDTH));

        let arguments_intro = "Available args:";
        println!("{}", format_text(arguments_intro, HELP_MESSAGE_MAX_LINE_WIDTH));

        let arguments = [
            ("-p <port>", format!("Set TCP port for the server. Default is {DEFAULT_PORT}.")),
            ("-e <boolean>", format!("Set whether the server should log every status received from clients or only when it changes. Default is {DEFAULT_LOG_EVERY_STATUS}.")),
            ("-h", "Print this message.".to_owned()),
            ("-v", "Print version.".to_owned()),
        ];
        println!(
            "{}",
            format_args_list(&arguments, HELP_MESSAGE_BASIC_INDENT_WIDTH, HELP_MESSAGE_MAX_LINE_WIDTH)
        );
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server_port: DEFAULT_PORT,
            log_every_status: DEFAULT_LOG_EVERY_STATUS,
            help: false,
            version: false,
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
