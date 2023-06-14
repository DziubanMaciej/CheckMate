use check_mate_common::{ReceiveCommandError, ServerCommand, ServerCommandError};
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
};
mod action;
mod config;

use action::Action;
use config::Config;

fn send_command(tcp_stream: &mut TcpStream, command: ServerCommand) -> Result<(), std::io::Error> {
    let buffer = command.to_bytes();
    match tcp_stream.write(&buffer) {
        Ok(_) => Ok(()),
        Err(err) => {
            eprintln!("Failed to write to tcp stream {}", err);
            std::process::exit(1);
        }
    }
}

fn receive_command(tcp_stream: &mut TcpStream) -> Result<ServerCommand, ReceiveCommandError> {
    const CHUNK_SIZE: usize = 32;

    let mut buffer = Vec::new();
    buffer.resize(CHUNK_SIZE, 0);
    let mut total_read_length = 0;

    loop {
        let read_length = tcp_stream.read(&mut buffer[total_read_length..]);
        let read_length = match read_length {
            Ok(x) => x,
            Err(err) => {
                eprintln!("Failed to read from tcp stream {}", err);
                std::process::exit(1);
            }
        };
        total_read_length += read_length;

        let error = match ServerCommand::from_bytes(&buffer[0..total_read_length]) {
            Ok(x) => break Ok(x.command),
            Err(err) => err,
        };

        match error {
            ServerCommandError::TooFewBytes => {
                buffer.resize(buffer.len() + CHUNK_SIZE, 0);
                continue;
            }
            _ => break Err(error.into()),
        };
    }
}

fn connect_to_server(server_address: SocketAddrV4) -> TcpStream {
    loop {
        let tcp_stream = match TcpStream::connect(server_address) {
            Ok(ok) => ok,
            Err(err) => {
                println!("Failed to connect with server: {}. Keep waiting.", err);
                std::thread::sleep(std::time::Duration::from_millis(500)); // TODO make this a parameter
                continue;
            }
        };
        return tcp_stream;
    }
}

fn execute_action(config: &Config, tcp_stream: &mut TcpStream) -> Result<(), std::io::Error> {
    if let Some(ref name) = config.client_name {
        let command = ServerCommand::SetName(name.clone());
        send_command(tcp_stream, command)?;
    }

    match &config.action {
        Action::ReadMessages(include_names) => {
            read_messages_from_server(tcp_stream, *include_names)
        }
        Action::WatchCommand(command, command_args) => {
            watch_command(tcp_stream, command, command_args)
        }
        Action::RefreshClientByName(name) => refresh_client_by_name(tcp_stream, name),
        Action::Abort => abort_server(tcp_stream),
    }
}

fn read_messages_from_server(
    tcp_stream: &mut TcpStream,
    include_names: bool,
) -> Result<(), std::io::Error> {
    send_command(tcp_stream, ServerCommand::GetStatuses(include_names)).unwrap(); // TODO remove unwrap
    match receive_command(tcp_stream).unwrap() {
        // TODO remove unwrap
        ServerCommand::Statuses(statuses) => {
            for status in statuses.iter() {
                println!("{}", status);
            }
        }
        _ => panic!("Unexpected command received"),
    }
    Ok(())
}

fn execute_command(command: &str, command_args: &Vec<String>) -> String {
    let subprocess = std::process::Command::new(command)
        .args(command_args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let subprocess = match subprocess {
        Ok(x) => x,
        Err(err) => return err.to_string(),
    };

    let subprocess_result = subprocess.wait_with_output();
    let subprocess_result = match subprocess_result {
        Ok(x) => x,
        Err(err) => return err.to_string(),
    };

    let subprocess_out = if subprocess_result.status.success() {
        subprocess_result.stdout
    } else {
        subprocess_result.stderr
    };

    let subprocess_out = String::from_utf8(subprocess_out);
    let subprocess_out = match subprocess_out {
        Ok(x) => x,
        Err(err) => return err.to_string(),
    };

    subprocess_out
}

fn watch_command(
    tcp_stream: &mut TcpStream,
    command: &str,
    command_args: &Vec<String>,
) -> Result<(), std::io::Error> {
    loop {
        let command_output = execute_command(command, command_args);
        let command_output = command_output
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(1)
            .next()
            .unwrap_or("")
            .to_string();
        let server_command = if command_output.is_empty() {
            ServerCommand::SetStatusOk
        } else {
            ServerCommand::SetStatusError(command_output)
        };

        send_command(tcp_stream, server_command)?;
        std::thread::sleep(std::time::Duration::from_millis(500)); // TODO make this a parameter
    }
}

fn refresh_client_by_name(_tcp_stream: &mut TcpStream, _name: &str) -> Result<(), std::io::Error> {
    todo!();
}

fn abort_server(tcp_stream: &mut TcpStream) -> Result<(), std::io::Error> {
    let command = ServerCommand::Abort;
    send_command(tcp_stream, command)
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

    let server_address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.server_port);
    let mut tcp_stream = connect_to_server(server_address);
    let action_result = execute_action(&config, &mut tcp_stream);
    if let Err(err) = action_result {
        println!("ERROR: {}", err);
        std::process::exit(1);
    }
}
