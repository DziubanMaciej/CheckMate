use std::string::FromUtf8Error;

/// Command sent from client to server
#[derive(Debug, PartialEq, Eq)]
pub enum ServerCommand {
    Abort,
    SetStatusOk,
    SetStatusError(String),
    GetStatuses(bool),
    RefreshClientByName(String),
    SetName(String),

    Statuses(Vec<String>),
    Refresh,
}

#[derive(Debug, PartialEq)]
pub enum ServerCommandError {
    TooFewBytes,
    InvalidStringEncoding,
    InvalidBoolean,
    UnknownCommand,
}

impl std::fmt::Display for ServerCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<FromUtf8Error> for ServerCommandError {
    fn from(_: FromUtf8Error) -> Self {
        ServerCommandError::InvalidStringEncoding
    }
}

impl ServerCommand {
    pub(crate) const ID_ABORT: u8 = 1;
    pub(crate) const ID_SET_STATUS_OK: u8 = 2;
    pub(crate) const ID_SET_STATUS_ERROR: u8 = 3;
    pub(crate) const ID_GET_STATUSES: u8 = 4;
    pub(crate) const ID_REFRESH_CLIENT_BY_NAME: u8 = 5;
    pub(crate) const ID_SET_NAME: u8 = 6;
    pub(crate) const ID_STATUSES: u8 = 7;
    pub(crate) const ID_REFRESH: u8 = 8;

    pub fn from_bytes(bytes: &[u8]) -> Result<ServerCommandParse, ServerCommandError> {
        let mut bytes_used = 0;

        let take_bytes = |index: &mut usize, count: usize| -> Result<&[u8], ServerCommandError> {
            if *index + count > bytes.len() {
                Err(ServerCommandError::TooFewBytes)
            } else {
                *index += count;
                Ok(&bytes[*index - count..*index])
            }
        };
        let take_bool = |index: &mut usize| -> Result<bool, ServerCommandError> {
            let b = take_bytes(index, 1)?;
            match b[0] {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(ServerCommandError::InvalidBoolean),
            }
        };
        let take_dword = |index: &mut usize| -> Result<u32, ServerCommandError> {
            let b = take_bytes(index, 4)?;
            let b: [u8; 4] = [b[0], b[1], b[2], b[3]]; // TODO why do I need this...
            let b = u32::from_ne_bytes(b);
            Ok(b)
        };
        let take_string = |index: &mut usize| -> Result<String, ServerCommandError> {
            let string_size = take_dword(index)?;
            let string = take_bytes(index, string_size as usize)?;
            let string = String::from_utf8(string.into())?;
            Ok(string)
        };
        let take_strings = |index: &mut usize| -> Result<Vec<String>, ServerCommandError> {
            let strings_size = take_dword(index)?;
            let mut strings: Vec<String> = Vec::new();
            for _ in 0..strings_size {
                strings.push(take_string(index)?);
            }
            Ok(strings)
        };

        let command_type = take_bytes(&mut bytes_used, 1)?[0];
        let command = match command_type {
            ServerCommand::ID_ABORT => ServerCommand::Abort,
            ServerCommand::ID_SET_STATUS_OK => ServerCommand::SetStatusOk,
            ServerCommand::ID_SET_STATUS_ERROR => {
                ServerCommand::SetStatusError(take_string(&mut bytes_used)?)
            }
            ServerCommand::ID_GET_STATUSES => {
                ServerCommand::GetStatuses(take_bool(&mut bytes_used)?)
            }
            ServerCommand::ID_REFRESH_CLIENT_BY_NAME => {
                ServerCommand::RefreshClientByName(take_string(&mut bytes_used)?)
            }
            ServerCommand::ID_SET_NAME => ServerCommand::SetName(take_string(&mut bytes_used)?),
            ServerCommand::ID_STATUSES => ServerCommand::Statuses(take_strings(&mut bytes_used)?),
            ServerCommand::ID_REFRESH => ServerCommand::Refresh,
            _ => return Err(ServerCommandError::UnknownCommand),
        };
        Ok(ServerCommandParse {
            command: command,
            bytes_used: bytes_used,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        fn append_strings(bytes: &mut Vec<u8>, strings: &Vec<String>) {
            let vector_len = &strings.len().to_le_bytes()[0..4];
            bytes.extend_from_slice(&vector_len);
            for string in strings {
                append_string(bytes, string)
            }
        }
        fn append_string(bytes: &mut Vec<u8>, string: &String) {
            let string_bytes = string.as_bytes();
            let string_len = &string_bytes.len().to_le_bytes()[0..4];
            bytes.extend_from_slice(&string_len);
            bytes.extend_from_slice(&string_bytes);
        }
        fn append_bool(bytes: &mut Vec<u8>, bool: &bool) {
            bytes.push(*bool as u8);
        }

        match self {
            ServerCommand::Abort => vec![ServerCommand::ID_ABORT],
            ServerCommand::SetStatusOk => vec![ServerCommand::ID_SET_STATUS_OK],
            ServerCommand::SetStatusError(message) => {
                let mut result = vec![ServerCommand::ID_SET_STATUS_ERROR];
                append_string(&mut result, message);
                result
            }
            ServerCommand::GetStatuses(include_names) => {
                let mut result = vec![ServerCommand::ID_GET_STATUSES];
                append_bool(&mut result, include_names);
                result
            }
            ServerCommand::RefreshClientByName(name) => {
                let mut result = vec![ServerCommand::ID_REFRESH_CLIENT_BY_NAME];
                append_string(&mut result, &name);
                result
            }
            ServerCommand::SetName(name) => {
                let mut result = vec![ServerCommand::ID_SET_NAME];
                append_string(&mut result, &name);
                result
            }
            ServerCommand::Statuses(statuses) => {
                let mut result = vec![ServerCommand::ID_STATUSES];
                append_strings(&mut result, statuses);
                result
            }
            ServerCommand::Refresh => vec![ServerCommand::ID_REFRESH],
        }
    }
}

#[derive(Debug)]
pub struct ServerCommandParse {
    pub command: ServerCommand,
    pub bytes_used: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_expected_serialized_string_length(s: &str) -> usize {
        let string_length_size = 4;
        let string_size = s.len(); // this will not work for non-ascii characters, but we won't be using them anyway
        string_length_size + string_size
    }

    fn get_expected_command_length_no_data() -> usize {
        1
    }

    fn get_expected_command_length_bool() -> usize {
        get_expected_command_length_no_data() + 1
    }

    fn get_expected_command_length_string(s: &str) -> usize {
        get_expected_command_length_no_data() + get_expected_serialized_string_length(s)
    }

    fn get_expected_command_length_string_vec(v: &Vec<String>) -> usize {
        let header_size = get_expected_command_length_no_data();
        let vec_length_size = 4;
        let strings_size: usize = v
            .iter()
            .map(|x| get_expected_serialized_string_length(x))
            .sum();
        header_size + vec_length_size + strings_size
    }

    #[test]
    fn command_abort_is_serialized() {
        let command = ServerCommand::Abort;
        let bytes = command.to_bytes();
        let parse_result = ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
        assert_eq!(parse_result.command, command);
        assert_eq!(parse_result.bytes_used, 1);
    }

    #[test]
    fn command_refresh_is_serialized() {
        let command = ServerCommand::Refresh;
        let bytes = command.to_bytes();
        let parse_result = ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
        assert_eq!(parse_result.command, command);
        assert_eq!(parse_result.bytes_used, 1);
    }

    #[test]
    fn command_set_status_ok_is_serialized() {
        let command = ServerCommand::SetStatusOk;
        let bytes = command.to_bytes();
        let parse_result = ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
        assert_eq!(parse_result.command, command);
        assert_eq!(parse_result.bytes_used, 1);
    }

    #[test]
    fn command_set_status_error_is_serialized() {
        let message = "Important error detected";
        let command = ServerCommand::SetStatusError(message.to_owned());
        let bytes = command.to_bytes();
        let parse_result = ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
        assert_eq!(parse_result.command, command);
        assert_eq!(
            parse_result.bytes_used,
            get_expected_command_length_string(message)
        );
    }

    #[test]
    fn command_get_statuses_is_serialized() {
        {
            let command = ServerCommand::GetStatuses(false);
            let bytes = command.to_bytes();
            let parse_result =
                ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
            assert_eq!(parse_result.command, command);
            assert_eq!(parse_result.bytes_used, get_expected_command_length_bool());
        }
        {
            let command = ServerCommand::GetStatuses(true);
            let bytes = command.to_bytes();
            let parse_result =
                ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
            assert_eq!(parse_result.command, command);
            assert_eq!(parse_result.bytes_used, get_expected_command_length_bool());
        }
    }

    #[test]
    fn command_refresh_client_by_name_is_serialized() {
        let name = "client12";
        let command = ServerCommand::RefreshClientByName(name.to_owned());
        let bytes = command.to_bytes();
        let parse_result = ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
        assert_eq!(parse_result.command, command);
        assert_eq!(
            parse_result.bytes_used,
            get_expected_command_length_string(name)
        );
    }

    #[test]
    fn command_set_name_is_serialized() {
        let name = "client12";
        let command = ServerCommand::SetName(name.to_owned());
        let bytes = command.to_bytes();
        let parse_result = ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
        assert_eq!(parse_result.command, command);
        assert_eq!(
            parse_result.bytes_used,
            get_expected_command_length_string(&name)
        );
    }

    #[test]
    fn command_statuses_is_serialized() {
        let statuses = vec!["err".to_owned(), "warn".to_owned(), "fail".to_owned()];
        let command = ServerCommand::Statuses(statuses.clone());
        let bytes = command.to_bytes();
        let parse_result = ServerCommand::from_bytes(&bytes).expect("Command should deserialize");
        assert_eq!(parse_result.command, command);
        assert_eq!(
            parse_result.bytes_used,
            get_expected_command_length_string_vec(&statuses)
        );
    }

    #[test]
    fn command_get_statuses_with_invalid_bool_should_fail() {
        let command = ServerCommand::GetStatuses(false);
        let mut bytes = command.to_bytes();
        bytes[1] = 2;
        let err = ServerCommand::from_bytes(&bytes)
            .expect_err("GetStatuses command with invalid bool should not be deserialized");
        assert_eq!(err, ServerCommandError::InvalidBoolean);
    }

    #[test]
    fn unknown_command_deserialization_fails() {
        let bytes = [7];
        ServerCommand::from_bytes(&bytes).expect_err("Unknown command should not be deserialized");
    }

    #[test]
    fn command_with_not_enough_bytes_should_fail() {
        let bytes = [ServerCommand::ID_SET_STATUS_ERROR];
        let err = ServerCommand::from_bytes(&bytes)
            .expect_err("Command with not enough bytes should not be deserialized");
        assert_eq!(err, ServerCommandError::TooFewBytes);

        let bytes = [ServerCommand::ID_REFRESH_CLIENT_BY_NAME];
        let err = ServerCommand::from_bytes(&bytes)
            .expect_err("Command with not enough bytes should not be deserialized");
        assert_eq!(err, ServerCommandError::TooFewBytes);
    }

    #[test]
    fn command_with_cut_string_should_fail() {
        let command = ServerCommand::SetStatusError("Important error detected".to_string());
        let bytes = command.to_bytes();

        let bytes = &bytes[0..bytes.len() - 1];
        let err: ServerCommandError = ServerCommand::from_bytes(&bytes)
            .expect_err("Command with not enough bytes should not be deserialized");
        assert_eq!(err, ServerCommandError::TooFewBytes);

        let bytes = &bytes[0..bytes.len() - 1];
        let err: ServerCommandError = ServerCommand::from_bytes(&bytes)
            .expect_err("Command with not enough bytes should not be deserialized");
        assert_eq!(err, ServerCommandError::TooFewBytes);
    }

    #[test]
    fn command_with_invalid_string_should_fail() {
        let bytes = [
            // Command type
            ServerCommand::ID_SET_STATUS_ERROR,
            // String length
            3,
            0,
            0,
            0,
            // Invalid utf string
            0xe2,
            0x28,
            0xa1,
        ];
        let err = ServerCommand::from_bytes(&bytes)
            .expect_err("Command with invalid utf8 string should fail");
        assert_eq!(err, ServerCommandError::InvalidStringEncoding);
    }
}
