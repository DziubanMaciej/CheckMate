use std::{
    net::{Ipv4Addr, SocketAddrV4, TcpStream},
    time::Duration,
};
mod action;
mod config;

use check_mate_common::CommunicationError;
use config::Config;

fn connect_to_server(server_address: SocketAddrV4, connection_backoff: Duration) -> TcpStream {
    loop {
        let tcp_stream = match TcpStream::connect(server_address) {
            Ok(ok) => ok,
            Err(err) => {
                eprintln!("Failed to connect with server: {}. Keep waiting.", err);
                if !connection_backoff.is_zero() {
                    std::thread::sleep(connection_backoff);
                }
                continue;
            }
        };
        return tcp_stream;
    }
}

fn main() {
    let config = Config::parse(std::env::args().skip(1));
    let config = match config {
        Ok(x) => x,
        Err(err) => {
            eprintln!("ERROR: {}", err);
            std::process::exit(1);
        }
    };

    let server_address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, config.server_port);

    loop {
        let mut tcp_stream = connect_to_server(server_address, config.server_connection_backoff);
        let action_result = config.action.execute(&mut tcp_stream, &config);

        if let Err(err) = action_result {
            match err {
                CommunicationError::ClientDisconnected => (),
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
