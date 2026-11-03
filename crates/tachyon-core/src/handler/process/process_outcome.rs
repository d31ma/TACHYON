use std::process::ExitStatus;

pub(super) enum ProcessOutcome {
    Exit(std::io::Result<ExitStatus>),
    Cancelled,
    TimedOut,
}
