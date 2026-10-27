use super::{Drained, ProcessFailure, WarmProcess};
use crate::Failure;
use crate::handler::frame::HandlerFrame;
use tokio::time::Instant;

pub(super) struct WarmExit;

impl WarmExit {
    pub(super) async fn failure(process: WarmProcess, deadline: Instant) -> Failure {
        let (status, stderr) = process.exit_report(deadline).await;
        let failed = status.filter(|status| !status.success());
        failed.map_or_else(Self::ended_early, |status| stderr.exit_failure(status))
    }

    pub(super) async fn stream_result(
        process: WarmProcess,
        deadline: Instant,
    ) -> Result<(), Failure> {
        let (status, stderr) = process.exit_report(deadline).await;
        let status = status.ok_or_else(Self::unobserved)?;
        Drained::validate_stream_output(status, &stderr)
    }

    fn ended_early() -> Failure {
        HandlerFrame::protocol_failure(
            2102,
            "Handler stdout ended before a frame prefix was received.",
        )
    }

    fn unobserved() -> Failure {
        ProcessFailure::stream("Cannot observe handler process exit.")
    }
}
