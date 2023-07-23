use super::definition::Action;
use check_mate_common::{CommunicationError, ServerCommand};
use tokio::io::AsyncWrite;

impl Action {
    pub(crate) async fn refresh_client_by_name(
        output_stream: &mut (impl AsyncWrite + Unpin),
        name: &str,
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::RefreshClientByName(name.into());
        command.send_async(output_stream).await
    }

    pub(crate) async fn refresh_all_clients(
        output_stream: &mut (impl AsyncWrite + Unpin),
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::RefreshAllClients;
        command.send_async(output_stream).await
    }
}
