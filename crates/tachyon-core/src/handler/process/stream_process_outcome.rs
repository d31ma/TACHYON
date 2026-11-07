use super::StreamReadOutcome;
use crate::Failure;
use std::process::ExitStatus;

pub(super) enum StreamProcessOutcome {
    Frames(Result<Result<StreamReadOutcome, Failure>, tokio::task::JoinError>),
    Exit(std::io::Result<ExitStatus>),
    SubscriberClosed,
    Cancelled,
    TimedOut,
}
