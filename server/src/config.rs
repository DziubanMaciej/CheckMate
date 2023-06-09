use check_mate_common::{fetch_arg, CommandLineError, DEFAULT_PORT};

#[derive(PartialEq, Debug)]
pub struct Config {
    pub server_port: u16,

}

impl Config {
    fn parse_options<T: Iterator<Item = String>>(&mut self, args: &mut T) -> Result<(), CommandLineError> {
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
                _ => return Err(CommandLineError::InvalidArgument(arg)),
            }
        }
        Ok(())
    }

    pub fn parse<T: Iterator<Item = String>>(mut args: T) -> Result<Config, CommandLineError> {
        let mut config = Config {
            server_port: DEFAULT_PORT,
        };
        config.parse_options(&mut args)?;
        Ok(config)
    }
}