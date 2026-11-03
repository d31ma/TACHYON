pub(in crate::server) enum WatchWait {
    Cancelled,
    Idle,
    Closed,
    Changed(Result<notify::Event, notify::Error>),
}
