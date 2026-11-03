use crate::handler::frame::HandlerFrame;
use command_group::AsyncGroupChild;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::process::ChildStdin;
use tokio::time::error::Elapsed;
use tokio::time::{Instant, timeout_at};

pub(in crate::handler) struct ProcessReaper;

impl ProcessReaper {
    const REAP_TIMEOUT: Duration = Duration::from_secs(1);

    pub(in crate::handler) async fn terminate(child: &mut AsyncGroupChild, request_id: &str) {
        let process_id = child.inner().id();
        let kill_failed = child.start_kill().is_err();
        let cleanup_deadline = Instant::now() + Self::REAP_TIMEOUT;
        let waited = timeout_at(cleanup_deadline, child.wait()).await;
        let reaped = Self::reaped(&waited);
        Self::report_unsettled(reaped, request_id, process_id, kill_failed);
    }

    pub(super) async fn cancel_and_reap(
        child: &mut AsyncGroupChild,
        stdin: Option<&mut ChildStdin>,
        request_id: &str,
        grace: Duration,
        deadline: Instant,
    ) {
        let grace_deadline = deadline.min(Instant::now() + grace);
        Self::send_cancel_frame(stdin, request_id, grace_deadline).await;
        Self::wait_within_grace(child, grace_deadline).await;
        Self::terminate(child, request_id).await;
    }

    pub(super) fn cleanup_unsettled_event(
        request_id: &str,
        process_id: Option<u32>,
        kill_failed: bool,
    ) -> serde_json::Value {
        serde_json::json!({
            "event": "handler.cleanup_unsettled",
            "request_id": request_id,
            "process_id": process_id,
            "kill_failed": kill_failed,
            "reap_failed": true,
        })
    }

    fn reaped(waited: &Result<std::io::Result<ExitStatus>, Elapsed>) -> bool {
        matches!(waited, Ok(Ok(_)))
    }

    fn report_unsettled(
        reaped: bool,
        request_id: &str,
        process_id: Option<u32>,
        kill_failed: bool,
    ) {
        if !reaped {
            eprintln!(
                "{}",
                Self::cleanup_unsettled_event(request_id, process_id, kill_failed)
            );
        }
    }

    async fn send_cancel_frame(
        stdin: Option<&mut ChildStdin>,
        request_id: &str,
        grace_deadline: Instant,
    ) {
        if let Some(stdin) = stdin
            && let Ok(frame) = HandlerFrame::cancel(request_id)
        {
            let _cancel = timeout_at(grace_deadline, async {
                let _write = stdin.write_all(&frame).await;
                let _flush = stdin.flush().await;
            })
            .await;
        }
    }

    async fn wait_within_grace(child: &mut AsyncGroupChild, grace_deadline: Instant) {
        if Instant::now() < grace_deadline {
            let _exit = timeout_at(grace_deadline, child.inner().wait()).await;
        }
    }
}
