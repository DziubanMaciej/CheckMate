use check_mate_common::{ReceiveCommandError, ServerCommand};
use std::io::BufRead;

pub struct ClientState<'a, T: BufRead> {
    input_stream: &'a mut T,
    name: Option<String>,
    status: Result<(), String>,
}

impl<'a, T: BufRead> ClientState<'a, T> {
    pub fn new(input_stream: &'a mut T) -> Self {
        ClientState {
            input_stream: input_stream,
            name: None,
            status: Ok(()),
        }
    }

    pub fn get_status(&self) -> &Result<(), String> {
        &self.status
    }

    pub fn get_name_for_logging(&self) -> String {
        // TODO rename
        self.name.clone().unwrap_or("<Unknown>".to_owned())
    }

    pub fn receive_command(&mut self) -> Result<Option<ServerCommand>, ReceiveCommandError> {
        let buffer = self.input_stream.fill_buf();
        let buffer = match buffer {
            Ok(x) => x,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::WouldBlock {
                    return Ok(None);
                } else {
                    return Err(ReceiveCommandError::from(err));
                }
            }
        };
        if buffer.len() == 0 {
            return Err(ReceiveCommandError::ClientDisconnected);
        }

        let parse_result = ServerCommand::from_bytes(buffer)?;
        self.input_stream.consume(parse_result.bytes_used);
        Ok(Some(parse_result.command))
    }

    pub fn process_command<ReadStatusesCb: FnMut(bool)>(
        &mut self,
        command: ServerCommand,
        mut on_read_statuses: ReadStatusesCb,
    ) {
        match command {
            ServerCommand::Abort => {
                println!("Received abort command");
                std::process::exit(0);
            }
            ServerCommand::SetStatusOk => {
                if let Err(_) = self.status {
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
                if is_new_error {
                    println!(
                        "Client {} has error: {}",
                        self.get_name_for_logging(),
                        self.status.as_ref().unwrap_err()
                    );
                }
            }
            ServerCommand::GetStatuses(include_names) => on_read_statuses(include_names),
            ServerCommand::RefreshClientByName(_) => panic!("Not implemented command"),
            ServerCommand::SetName(name) => {
                println!("Name set to {}", name);
                self.name = Some(name);
            }
            ServerCommand::Statuses(_) => panic!("Unexpected message received"),
        };
    }
}
