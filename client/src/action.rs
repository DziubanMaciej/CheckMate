#[derive(PartialEq, Debug)]
pub enum Action {
    ReadMessages(bool),
    WatchCommand(String, Vec<String>),
    RefreshClientByName(String),
    Abort,
}
