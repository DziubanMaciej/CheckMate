use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::{io::BufReader, net::TcpStream};
mod action;
mod config;

use check_mate_common::{constants::*, CommunicationError};
use config::Config;

async fn connect_to_server(
    server_address: SocketAddrV4,
    connection_backoff: Duration,
    connection_attemps: u32,
) -> Option<TcpStream> {
    let mut attempts_made: u32 = 0;
    loop {
        attempts_made += 1;
        match TcpStream::connect(server_address).await {
            Ok(ok) => break Some(ok),
            Err(err) => {
                if connection_attemps > 0 && attempts_made == connection_attemps {
                    break None;
                }
                eprintln!("Failed to connect with server: {}. Keep waiting.", err);
                tokio::time::sleep(connection_backoff).await;
            }
        };
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

    // Handle simple actions, which do not require connecting to the server
    match config.action {
        action::Action::Help => {
            Config::print_help();
            std::process::exit(0);
        }
        action::Action::Version => {
            println!("{VERSION}");
            std::process::exit(0);
        }
        _ => (),
    }

    let server_address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.server_port);

    loop {
        // Connect to server
        let tcp_stream = connect_to_server(
            server_address,
            config.server_connection_backoff,
            config.server_connection_attempts,
        )
        .await;
        let tcp_stream = match tcp_stream {
            Some(some) => some,
            None => {
                eprintln!("Failed to connect with server. Aborting.");
                std::process::exit(1);
            }
        };

        // Prepare IO streams
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
