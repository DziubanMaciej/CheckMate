#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages,
    WatchCommand(String, Vec<String>),
    RefreshClientByName(String),
    Abort,
}
