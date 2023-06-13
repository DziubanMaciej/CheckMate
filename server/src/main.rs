mod client_state;
mod config;
mod thread_communication;

use check_mate_common::{ReceiveCommandError, ServerCommand};
use client_state::ClientState;
use config::Config;
use std::io::Write;
use std::{
    io::BufReader,
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    sync::mpsc,
};
use thread_communication::ThreadCommunication;

fn send_command<T: Write>(
    tcp_stream: &mut T,
    command: ServerCommand,
) -> Result<(), std::io::Error> {
    let buffer = command.to_bytes();
    tcp_stream.write(&buffer)?;
    Ok(())
}

fn handle_client(mut thread_communication: ThreadCommunication, tcp_stream: TcpStream) {
    // Initialize communication with our client
    tcp_stream
        .set_nonblocking(true)
        .expect("Cannot use nonblocking sockets");
    let mut input_stream = BufReader::new(tcp_stream.try_clone().unwrap());
    let mut output_stream = tcp_stream.try_clone().unwrap();
    let mut client_state = ClientState::new(&mut input_stream);

    // Initialize communication with other server threads
    let (sender, mut receiver) = mpsc::channel();
    thread_communication.add_current_thread(sender);

    // Main loop
    let main_loop_result: Result<(), ReceiveCommandError> = loop {
        let on_read_statuses = || {
            let errors = thread_communication.read_messages(&receiver);
            let command = ServerCommand::Statuses(errors);
            let _send_result = send_command(&mut output_stream, command); // ignore result - if it failed, client must have died
        };

        // Communicate with client
        let command = match client_state.receive_command() {
            Ok(x) => x,
            Err(err) => break Err(err),
        };
        if let Some(command) = command {
            client_state.process_command(command, on_read_statuses);
        }

        // Communicate with other threads
        thread_communication.process_messages_from_other_threads(&mut receiver, &mut client_state)
    };

    // Handle errors
    if let Err(err) = main_loop_result {
        match err {
            ReceiveCommandError::CommandParseError(_command_err) => {
                println!("Failed to parse commands from client")
            }
            ReceiveCommandError::IoError(io_err) => {
                println!("Failed to receive commands from client: {}", io_err)
            }
            ReceiveCommandError::ClientDisconnected => {}
        }
    }

    thread_communication.remove_current_thread();
}

fn main() {
    let config = Config::parse(std::env::args().skip(1));
    let config = match config {
        Ok(x) => x,
        Err(err) => {
            println!("ERROR: {}", err);
            std::process::exit(1);
        }
    };

    let socket_address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.server_port);
    let listener = TcpListener::bind(socket_address);
    let listener = listener.unwrap_or_else(|err| {
        eprintln!("Failed to bind address: {}", err);
        std::process::exit(1);
    });

    let thread_communication = ThreadCommunication::new();

    for tcp_stream in listener.incoming() {
        let tcp_stream = match tcp_stream {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("Failed to connect with client: {}", err);
                continue;
            }
        };

        let thread_communication = thread_communication.clone();
        std::thread::spawn(|| handle_client(thread_communication, tcp_stream));
    }
}
