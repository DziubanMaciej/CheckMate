use super::watch_action::WatchCommandData;
use crate::config::Config;
use check_mate_common::{CommunicationError, ServerCommand};
use tokio::io::{AsyncBufRead, AsyncWrite};

#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages(bool),
    WatchCommand(WatchCommandData),
    RefreshClientByName(String),
    RefreshAllClients,
    ListClients,
    Abort,
    Help,
    Version,
}

impl Action {
    pub fn should_reconnect(&self) -> bool {
        matches!(self, Self::WatchCommand(_))
    }

    pub async fn execute(
        &self,
        input_stream: &mut (impl AsyncBufRead + Unpin),
        output_stream: &mut (impl AsyncWrite + Unpin),
        config: &Config,
    ) -> Result<(), CommunicationError> {
        if let Some(ref name) = config.client_name {
            let command = ServerCommand::SetName(name.clone());
            command.send_async(output_stream).await?;
        }

        match self {
            Action::ReadMessages(include_names) => {
                Self::read(input_stream, output_stream, *include_names).await
            }
            Action::WatchCommand(data) => Self::watch(input_stream, output_stream, data).await,
            Action::RefreshClientByName(name) => {
                Self::refresh_client_by_name(output_stream, name).await
            }
            Action::RefreshAllClients => Self::refresh_all_clients(output_stream).await,
            Action::ListClients => Self::list_clients(input_stream, output_stream).await,
            Action::Abort => Self::abort(output_stream).await,
            Action::Help => panic!("Cannot execute help action"),
            Action::Version => panic!("Cannot execute version action"),
        }
    }
}
