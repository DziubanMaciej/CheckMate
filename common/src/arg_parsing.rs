use std::str::FromStr;

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

pub fn fetch_arg_and_parse<Int, T, U, V>(
    args: &mut T,
    on_fetch_error: U,
    on_conversion_error: V,
) -> Result<Int, CommandLineError>
where
    T: Iterator<Item = String>,
    U: Fn() -> CommandLineError,
    V: Fn(&str) -> CommandLineError,
    Int : FromStr,
{
    let arg = match args.next() {
        Some(x) => x,
        None => return Err(on_fetch_error()),
    };

    let arg = match arg.parse::<Int>() {
        Ok(x) => x,
        Err(_) => return Err(on_conversion_error(&arg)),
    };

    Ok(arg)
}

pub fn fetch_arg_bool<T, U, V>(
    args: &mut T,
    on_fetch_error: U,
    on_conversion_error: V,
) -> Result<bool, CommandLineError>
where
    T: Iterator<Item = String>,
    U: Fn() -> CommandLineError,
    V: Fn(&str) -> CommandLineError,
{
    let arg = match args.next() {
        Some(x) => x,
        None => return Err(on_fetch_error()),
    };

    match arg.as_ref() {
        "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        _ => Err(on_conversion_error(&arg)),
    }
}

pub fn fetch_arg_string<T, U, V>(
    args: &mut T,
    on_fetch_error: U,
    on_empty_string: V,
) -> Result<String, CommandLineError>
where
    T: Iterator<Item = String>,
    U: Fn() -> CommandLineError,
    V: Fn() -> CommandLineError,
{
    let arg = match args.next() {
        Some(x) => x,
        None => return Err(on_fetch_error()),
    };

    if arg.is_empty() {
        return Err(on_empty_string());
    }

    Ok(arg)
}
