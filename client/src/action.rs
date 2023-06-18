use std::io::{Read, Write};

use check_mate_common::{CommunicationError, ServerCommand};

#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages(bool),
    WatchCommand(String, Vec<String>),
    RefreshClientByName(String),
    Abort,
}

impl Action {
    pub fn should_reconnect(&self) -> bool {
        match self {
            Self::WatchCommand(_, _) => true,
            _ => false,
        }
    }

    pub fn execute<T>(
        &self,
        tcp_stream: &mut T,
        client_name: &Option<String>,
    ) -> Result<(), CommunicationError>
    where
        T: Read + Write,
    {
        if let Some(ref name) = client_name {
            let command = ServerCommand::SetName(name.clone());
            command.send(tcp_stream, true)?;
        }

        match self {
            Action::ReadMessages(include_names) => Self::read(tcp_stream, *include_names),
            Action::WatchCommand(command, command_args) => {
                Self::watch(tcp_stream, command, command_args)
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
            std::thread::sleep(std::time::Duration::from_millis(500)); // TODO make this a parameter
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
