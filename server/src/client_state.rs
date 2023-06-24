use check_mate_common::ServerCommand;
use tokio::sync::mpsc::{channel, Receiver, Sender};

pub struct ClientState {
    log_every_status: bool,
    name: Option<String>,
    status: Result<(), String>,
    messages_to_send_queue: (Sender<ServerCommand>, Receiver<ServerCommand>),
}

pub enum ProcessCommandResult {
    Ok,
    GetStatuses(bool),
    RefreshClientByName(String),
}

impl ClientState {
    pub fn new(log_every_status: bool) -> Self {
        ClientState {
            log_every_status: log_every_status,
            name: None,
            status: Ok(()),
            messages_to_send_queue: channel(2),
        }
    }

    pub fn get_status(&self) -> &Result<(), String> {
        &self.status
    }

    pub fn get_name(&self) -> &Option<String> {
        &self.name
    }

    pub fn get_name_for_logging(&self) -> String {
        // TODO rename
        self.name.clone().unwrap_or("<Unknown>".to_owned())
    }

    pub async fn push_command_to_send(&mut self, command: ServerCommand) {
        self.messages_to_send_queue
            .0
            .send(command)
            .await
            .expect("Receiver inside ClientState should never be destroyed");
    }

    pub async fn get_command_to_send(&mut self) -> ServerCommand {
        self.messages_to_send_queue
            .1
            .recv()
            .await
            .expect("Sender inside ClientState should never be destroyed")
    }

    pub fn process_command(&mut self, command: ServerCommand) -> ProcessCommandResult {
        match command {
            ServerCommand::Abort => {
                println!("Received abort command");
                std::process::exit(0);
            }
            ServerCommand::SetStatusOk => {
                if self.log_every_status || self.status.is_err() {
                    println!("Client {} is ok", self.get_name_for_logging());
                }
                self.status = Ok(());
            }
            ServerCommand::SetStatusError(new_err) => {
                let is_new_error = match self.status {
                    Ok(_) => true,
                    Err(ref old_err) => *old_err != new_err,
                };
                self.status = Err(new_err);
                if self.log_every_status || is_new_error {
                    println!(
                        "Client {} has error: {}",
                        self.get_name_for_logging(),
                        self.status.as_ref().unwrap_err()
                    );
                }
            }
            ServerCommand::GetStatuses(include_names) => {
                return ProcessCommandResult::GetStatuses(include_names)
            }
            ServerCommand::RefreshClientByName(name) => {
                return ProcessCommandResult::RefreshClientByName(name)
            }
            ServerCommand::SetName(name) => {
                println!("Name set to {}", name);
                self.name = Some(name);
            }
            ServerCommand::Statuses(_) => panic!("Unexpected server command"),
            ServerCommand::Refresh => panic!("Unexpected server command"),
        };

        ProcessCommandResult::Ok
    }
}
