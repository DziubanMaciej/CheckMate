use super::definition::Action;
use check_mate_common::{CommunicationError, ServerCommand};
use tokio::io::{AsyncBufRead, AsyncWrite};

impl Action {
    pub(crate) async fn list_clients(
        input_stream: &mut (impl AsyncBufRead + Unpin),
        output_stream: &mut (impl AsyncWrite + Unpin)
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::ListClients;
        command.send_async(output_stream).await?;

        match ServerCommand::receive_async(input_stream).await? {
            ServerCommand::Clients(clients) => {
                for client in clients {
                    println!("{}", client);
                }
            }
            _ => panic!("Unexpected command received after ListClients"),
        }
        Ok(())
    }
}
