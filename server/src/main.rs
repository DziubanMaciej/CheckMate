use check_mate_common::{ServerCommand, ServerCommandError, DEFAULT_PORT};
use core::panic;
use std::{
    io::{BufRead, BufReader},
    net::{Ipv4Addr, SocketAddrV4, TcpListener},
};

enum ReceiveCommandError {
    IoError(std::io::Error),
    CommandParseError(ServerCommandError),
    ClientDisconnected,
}

impl From<std::io::Error> for ReceiveCommandError {
    fn from(err: std::io::Error) -> Self {
        ReceiveCommandError::IoError(err)
    }
}

impl From<ServerCommandError> for ReceiveCommandError {
    fn from(err: ServerCommandError) -> Self {
        ReceiveCommandError::CommandParseError(err)
    }
}

fn receive_command<T: BufRead>(input_stream: &mut T) -> Result<ServerCommand, ReceiveCommandError> {
    let buffer = input_stream.fill_buf()?;
    if buffer.len() == 0 {
        return Err(ReceiveCommandError::ClientDisconnected);
    }

    let parse_result = ServerCommand::from_bytes(buffer)?;
    input_stream.consume(parse_result.bytes_used);
    Ok(parse_result.command)
}

struct ClientState<'a, T: BufRead> {
    input_stream: &'a mut T,
    name: Option<String>,
    status: Result<(), String>,
}

impl<'a, T: BufRead> ClientState<'a, T> {
    fn new(input_stream: &'a mut T) -> Self {
        ClientState {
            input_stream: input_stream,
            name: None,
            status: Ok(()),
        }
    }

    fn get_name_for_logging(&self) -> String {
        self.name.clone().unwrap_or("<Unknown>".to_owned())
    }

    fn process_commands(&mut self) -> Result<(), ReceiveCommandError> {
        loop {
            let command = receive_command(self.input_stream)?;
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
                ServerCommand::GetStatuses => panic!("Not implemented command"),
                ServerCommand::RefreshClientByName(_) => panic!("Not implemented command"),
                ServerCommand::SetName(name) => {
                    println!("Name set to {}", name);
                    self.name = Some(name);
                }
            };
        }
    }
}

fn main() {
    let port = DEFAULT_PORT;
    let socket_address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);

    let listener = TcpListener::bind(socket_address);
    let listener = listener.unwrap_or_else(|err| {
        eprintln!("Failed to bind address: {}", err);
        std::process::exit(1);
    });

    for tcp_stream in listener.incoming() {
        let tcp_stream = match tcp_stream {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("Failed to connect with client: {}", err);
                continue;
            }
        };
        let mut input_stream = BufReader::new(tcp_stream);
        let mut client_state = ClientState::new(&mut input_stream);

        if let Err(err) = client_state.process_commands() {
            match err {
                ReceiveCommandError::CommandParseError(_command_err) => {
                    println!("Failed to parse commands from client")
                }
                ReceiveCommandError::IoError(io_err) => {
                    println!("Failed to receive commands from client: {}", io_err)
                }
                ReceiveCommandError::ClientDisconnected => continue,
            }
        }
    }
}
