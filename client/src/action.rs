#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages,
    WatchCommand(String),
    RefreshClientByName(String),
    Abort,
}
