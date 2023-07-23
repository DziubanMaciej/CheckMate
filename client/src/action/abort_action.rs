use super::definition::Action;
use check_mate_common::{CommunicationError, ServerCommand};
use tokio::io::AsyncWrite;

impl Action {
    pub(crate) async fn abort(
        output_stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::Abort;
        command.send_async(output_stream).await
    }
}
