#[derive(PartialEq, Debug)]
pub enum CommandLineError {
    NoValueSpecified(String, String),
    InvalidValue(String, String),
    InvalidArgument(String),
}

impl std::fmt::Display for CommandLineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::NoValueSpecified(name, option) => {
                write!(f, "Specify a {} after {}", name, option)
            }
            Self::InvalidValue(name, value) => {
                write!(f, "Invalid {} value specified: {}", name, value)
            }
            Self::InvalidArgument(arg) => write!(f, "Invalid argument specified: {}", arg),
        }?;
        Ok(())
    }
}

pub fn fetch_arg<T: Iterator<Item = String>>(
    args: &mut T,
    on_error: CommandLineError,
) -> Result<String, CommandLineError> {
    match args.next() {
        Some(x) => Ok(x),
        None => return Err(on_error),
    }
}
