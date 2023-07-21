use std::time::Duration;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub const HELP_MESSAGE_MAX_LINE_WIDTH: usize = 120;
pub const HELP_MESSAGE_BASIC_INDENT_WIDTH: usize = 2;

pub const DEFAULT_PORT: u16 = 10005;
pub const DEFAULT_CONNECTION_BACKOFF: Duration = Duration::from_millis(500);
pub const DEFAULT_WATCH_INTERVAL: Duration = Duration::from_millis(1000);
pub const DEFAULT_WATCH_DELAY: Duration = Duration::from_millis(0);
pub const DEFAULT_INCLUDE_NAMES: bool = false;
pub const DEFAULT_SHELL: bool = false;
pub const DEFAULT_LOG_EVERY_STATUS: bool = false;
pub const DEFAULT_MAXIMUM_SERVER_CONNECTION_ATTEMPTS: u32 = 0;
