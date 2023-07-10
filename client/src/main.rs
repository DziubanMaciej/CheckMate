use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::{io::BufReader, net::TcpStream};
mod action;
mod config;

use check_mate_common::CommunicationError;
use config::Config;

async fn connect_to_server(
    server_address: SocketAddrV4,
    connection_backoff: Duration,
) -> TcpStream {
    loop {
        let tcp_stream = match TcpStream::connect(server_address).await {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("Failed to connect with server: {}. Keep waiting.", err);
                if !connection_backoff.is_zero() {
                    tokio::time::sleep(connection_backoff).await;
                }
                continue;
            }
        };
        return tcp_stream;
    }
}

#[tokio::main]
async fn main() {
    let config = Config::parse(std::env::args().skip(1));
    let config = match config {
        Ok(x) => x,
        Err(err) => {
            eprintln!("ERROR: {}", err);
            eprintln!("Use help action for more information.");
            std::process::exit(1);
        }
    };

    if matches!(config.action, action::Action::Help) {
        Config::print_help();
        std::process::exit(0);
    }

    let server_address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.server_port);

    loop {
        // Connect to server and prepare IO streams
        let tcp_stream = connect_to_server(server_address, config.server_connection_backoff).await;
        let (input_stream, mut output_stream) = tcp_stream.into_split();
        let mut input_stream = BufReader::new(input_stream);

        // Execute action
        let action_result = config
            .action
            .execute(&mut input_stream, &mut output_stream, &config)
            .await;

        // Handle errors
        if let Err(err) = action_result {
            match err {
                CommunicationError::SocketDisconnected => (),
                _ => {
                    eprintln!("ERROR: {}", err);
                    std::process::exit(1);
                }
            }
        }

        if !config.action.should_reconnect() {
            break;
        }
    }
}
