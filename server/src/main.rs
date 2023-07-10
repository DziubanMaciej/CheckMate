mod client_state;
mod config;
mod task_communication;

use check_mate_common::ServerCommand;
use client_state::ClientState;
use config::Config;
use std::net::{Ipv4Addr, SocketAddrV4};
use task_communication::{TaskCommunication, TaskMessage};
use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::sync::mpsc::{channel, Receiver, Sender};

async fn execute_command_from_client(
    task_id: usize,
    client_state: &mut ClientState,
    receiver: &mut Receiver<TaskMessage>,
    sender: &Sender<TaskMessage>,
    task_communication: &mut TaskCommunication,

    command: ServerCommand,
) {
    match client_state.process_command(command) {
        client_state::ProcessCommandResult::Ok => (),
        client_state::ProcessCommandResult::GetStatuses(include_names) => {
            let errors = task_communication
                .read_messages(task_id, receiver, sender, include_names)
                .await;
            client_state
                .push_command_to_send(ServerCommand::Statuses(errors))
                .await;
        }
        client_state::ProcessCommandResult::RefreshClientByName(name) => {
            task_communication
                .refresh_client_by_name(task_id, name)
                .await;
        }
        client_state::ProcessCommandResult::RefreshAllClients => {
            task_communication
                .refresh_all_clients(task_id)
                .await;
        }
    }
}

async fn handle_client_async(
    task_id: usize,
    mut task_communication: TaskCommunication,
    config: Config,
    stream: tokio::net::TcpStream,
) {
    // Prepare communication with client
    let (input_stream, mut output_stream) = stream.into_split();
    let mut input_stream = BufReader::new(input_stream);

    let (sender, mut receiver) = channel::<task_communication::TaskMessage>(1);
    task_communication
        .register_task(task_id, sender.clone())
        .await;

    let mut client_state = ClientState::new(config.log_every_status);

    let mut buffer: Vec<u8> = Vec::new();
    buffer.resize(100, 0);

    // Main loop
    let _err = loop {
        tokio::select! {
            command = ServerCommand::receive_async(&mut input_stream) => {
                match command {
                    Ok(x) => execute_command_from_client(task_id, &mut client_state, &mut receiver, &sender, &mut task_communication, x).await,
                    Err(x) => break x,
                };
            }
            task_message = receiver.recv() => {
                match task_message {
                    Some(x) => task_communication.process_task_message(x, &mut client_state).await,
                    None => todo!(), // TODO what does it mean?
                }
            }
            command = client_state.get_command_to_send() => {
                match command.send_async(&mut output_stream).await{
                    Ok(_) => (),
                    Err(x) => break x,
                }
            }
        }
    };

    // TODO: handle error

    task_communication.unregister_task(task_id).await;
}

#[tokio::main]
async fn main() {
    let config = Config::parse(std::env::args().skip(1));
    let config = match config {
        Ok(x) => x,
        Err(err) => {
            println!("ERROR: {}", err);
            std::process::exit(1);
        }
    };

    if config.help {
        Config::print_help();
        std::process::exit(0);
    }

    let mut task_id: usize = 0;

    let socket_address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.server_port);
    let listener = TcpListener::bind(socket_address);
    let listener = listener.await.unwrap_or_else(|err| {
        eprintln!("Failed to bind address: {}", err);
        std::process::exit(1);
    });

    let task_communication = TaskCommunication::new();

    loop {
        let tcp_stream = listener.accept().await;
        let (tcp_stream, _client_address) = match tcp_stream {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("Failed to connect with client: {}", err);
                continue;
            }
        };

        let task_communication = task_communication.clone();
        let config = config.clone();
        tokio::spawn(async move {
            handle_client_async(task_id, task_communication, config, tcp_stream).await;
        });

        task_id += 1;
    }
}
