use check_mate_common::{CommunicationError, ServerCommand, ServerCommandError};
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub async fn receive_blocking_async<T: AsyncBufRead + Unpin>(
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
