use crate::server_command::ServerCommandError;

#[derive(Debug)]
pub enum ReceiveCommandError {
    IoError(std::io::Error),
    CommandParseError(ServerCommandError),
    ClientDisconnected,
}

impl From<std::io::Error> for ReceiveCommandError {
    fn from(err: std::io::Error) -> Self {
        ReceiveCommandError::IoError(err)
    }
}

impl From<ServerCommandError> for ReceiveCommandError {
    fn from(err: ServerCommandError) -> Self {
        ReceiveCommandError::CommandParseError(err)
    }
}
