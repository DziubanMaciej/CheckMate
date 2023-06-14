use std::fmt::Display;

use crate::server_command::ServerCommandError;

#[derive(Debug)]
pub enum CommunicationError {
    IoError(std::io::Error),
    CommandParseError(ServerCommandError),
    ClientDisconnected,
}

impl From<std::io::Error> for CommunicationError {
    fn from(err: std::io::Error) -> Self {
        CommunicationError::IoError(err)
    }
}

impl From<ServerCommandError> for CommunicationError {
    fn from(err: ServerCommandError) -> Self {
        CommunicationError::CommandParseError(err)
    }
}

impl Display for CommunicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommunicationError::IoError(err) => write!(f, "IoError {}", err),
            CommunicationError::ClientDisconnected => write!(f, "Client disconnected"),
            CommunicationError::CommandParseError(err) => write!(f, "CommandParseError {}", err),
        }
    }
}
