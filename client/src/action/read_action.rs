use super::definition::Action;
use check_mate_common::{CommunicationError, ServerCommand};
use tokio::io::{AsyncBufRead, AsyncWrite};

impl Action {
    pub(crate) async fn read(
        input_stream: &mut (impl AsyncBufRead + Unpin),
        output_stream: &mut (impl AsyncWrite + Unpin),
        include_names: bool,
    ) -> Result<(), CommunicationError> {
        let command = ServerCommand::GetStatuses(include_names);
        command.send_async(output_stream).await?;

        match ServerCommand::receive_async(input_stream).await? {
            ServerCommand::Statuses(statuses) => {
                let mut iter = statuses.iter().peekable();
                while let Some(status) = iter.next() {
                    println!("{}", status);
                    if iter.peek().is_some() {
                        println!();
                    }
                }
            }
            _ => panic!("Unexpected command received after GetStatuses"),
        }
        Ok(())
    }
}
