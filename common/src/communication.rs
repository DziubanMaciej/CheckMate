use std::fmt::Display;

use crate::server_command::{ServerCommand, ServerCommandError};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

#[derive(Debug)]
pub enum CommunicationError {
    IoError(std::io::Error),
    CommandParseError(ServerCommandError),
    ClientDisconnected, // TODO rename to SocketDisconnected
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

impl ServerCommand {
    pub async fn receive_async<T: AsyncBufRead + Unpin>(
        input_stream: &mut T,
    ) -> Result<ServerCommand, CommunicationError> {
        loop {
            let buffer = input_stream.fill_buf().await?;
            if buffer.len() == 0 {
                return Err(CommunicationError::ClientDisconnected);
            }

            match ServerCommand::from_bytes(&buffer) {
                Ok(parse_result) => {
                    input_stream.consume(parse_result.bytes_used);
                    break Ok(parse_result.command);
                }
                Err(err) => match err {
                    ServerCommandError::TooFewBytes => continue,
                    _ => break Err(err.into()),
                },
            }
        }
    }

    pub async fn send_async(
        &self,
        stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), CommunicationError> {
        let command_bytes = self.to_bytes();
        match stream.write(&command_bytes[0..]).await {
            Ok(_) => Ok(()),
            Err(_) => Err(CommunicationError::ClientDisconnected),
        }
    }
}
