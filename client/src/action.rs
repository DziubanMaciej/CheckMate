use crate::config::Config;
use check_mate_common::{CommunicationError, ServerCommand, DEFAULT_WATCH_INTERVAL};
use std::{
    io::{Read, Write},
    time::Duration,
};

#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages(bool),
    WatchCommand(WatchCommandData),
    RefreshClientByName(String),
    Abort,
}

#[derive(PartialEq, Debug)]
pub struct WatchCommandData {
    pub command: String,
    pub command_args: Vec<String>,
    pub interval: Duration,
}

impl WatchCommandData {
    pub fn new(command: String, command_args: Vec<String>) -> Self {
        Self {
            command,
            command_args,
            interval: DEFAULT_WATCH_INTERVAL,
        }
    }
}

impl Action {
    pub fn should_reconnect(&self) -> bool {
        match self {
            Self::WatchCommand(_) => true,
            _ => false,
        }
    }

    pub fn execute<T>(&self, tcp_stream: &mut T, config: &Config) -> Result<(), CommunicationError>
    where
        T: Read + Write,
    {
        if let Some(ref name) = config.client_name {
            let command = ServerCommand::SetName(name.clone());
            command.send(tcp_stream, true)?;
        }

        match self {
            Action::ReadMessages(include_names) => Self::read(tcp_stream, *include_names),
            Action::WatchCommand(data) => {
                Self::watch(tcp_stream, &data.command, &data.command_args, data.interval)
            }
            Action::RefreshClientByName(name) => Self::refresh_client_by_name(tcp_stream, name),
            Action::Abort => Self::abort(tcp_stream),
        }
    }

    fn read<T>(input_output_stream: &mut T, include_names: bool) -> Result<(), CommunicationError>
    where
        T: Read + Write,
    {
        ServerCommand::GetStatuses(include_names).send(input_output_stream, true)?;

        match ServerCommand::receive_blocking(input_output_stream)? {
            ServerCommand::Statuses(statuses) => {
                for status in statuses.iter() {
                    println!("{}", status);
                }
            }
            _ => panic!("Unexpected command received"),
        }
        Ok(())
    }

    fn watch<T>(
        output_stream: &mut T,
        command: &str,
        command_args: &Vec<String>,
        interval : Duration,
    ) -> Result<(), CommunicationError>
    where
        T: Write,
    {
        loop {
            let command_output = Self::execute_command(command, command_args);
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

            server_command.send(output_stream, false)?;
            if !interval.is_zero() {
                std::thread::sleep(interval);
            }
        }
    }

    fn refresh_client_by_name<T>(
        _output_stream: &mut T,
        _name: &str,
    ) -> Result<(), CommunicationError>
    where
        T: Write,
    {
        todo!();
    }

    fn abort<T>(output_stream: &mut T) -> Result<(), CommunicationError>
    where
        T: Write,
    {
        let command = ServerCommand::Abort;
        command.send(output_stream, true)
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
}
