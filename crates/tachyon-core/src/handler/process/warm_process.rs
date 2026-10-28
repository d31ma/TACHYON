use super::{
    AdapterFiles, Drained, HandlerSupervisor, ProcessReaper, WarmFrames, WarmKey, WarmOutput,
    WarmReady, WarmStart, WarmStderr, WarmStop,
};
use crate::Failure;
use crate::handler::HandlerSource;
use crate::handler::frame::FramePrefix;
use command_group::AsyncGroupChild;
use futures_util::future::OptionFuture;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::process::ChildStdin;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio::time::{Instant, timeout_at};

pub(super) struct WarmProcess {
    key: WarmKey,
    child: AsyncGroupChild,
    stdin: ChildStdin,
    output: mpsc::Receiver<WarmOutput>,
    stderr: WarmStderr,
    served: u32,
    announced: bool,
    idle_since: Instant,
    reader: JoinHandle<()>,
    _source: HandlerSource,
    _adapter: AdapterFiles,
}

impl WarmProcess {
    pub(super) const SWITCH: &'static str = "YON_HANDLER_WARM";
    pub(super) const LABEL: &'static str = "warm_process";

    pub(super) fn start(start: WarmStart) -> Self {
        let style = FramePrefix::of(&start.source);
        let (output, reader) = WarmFrames::spawn(start.stdout, style);
        Self {
            key: WarmKey::of(&start.source),
            child: start.child,
            stdin: start.stdin,
            output,
            stderr: WarmStderr::spawn(start.stderr, start.stderr_limit),
            served: 0,
            announced: false,
            idle_since: Instant::now(),
            reader,
            _source: start.source,
            _adapter: start.adapter,
        }
    }

    pub(super) fn serves(&self, key: &WarmKey) -> bool {
        self.key == *key
    }

    pub(super) fn is_ready(&mut self) -> bool {
        let quiet = self.output.is_empty();
        let listening = !self.output.is_closed();
        let running = self.still_running();
        [quiet, listening, running] == [true, true, true]
    }

    pub(super) async fn into_ready(self) -> Option<Self> {
        self.ready_or_retired().await
    }

    pub(super) fn idle_since(&self) -> Instant {
        self.idle_since
    }

    pub(super) fn idle_for(&self, now: Instant) -> Duration {
        now.saturating_duration_since(self.idle_since)
    }

    pub(super) fn has_served(&self, limit: u32) -> bool {
        self.served >= limit
    }

    pub(super) fn begin_request(&mut self) {
        let _earlier = self.stderr.take();
    }

    pub(super) fn finish_request(&mut self) {
        self.served = self.served.saturating_add(1);
        self.idle_since = Instant::now();
    }

    /// Waits for a new process's ready frame; a process that already said it is ready goes on.
    pub(super) async fn until_ready(&mut self) -> Result<(), WarmStop> {
        let waiting = !self.announced;
        let first = OptionFuture::from(waiting.then(|| self.output.recv())).await;
        self.announced = true;
        first.map_or(Ok(()), WarmReady::check)
    }

    pub(super) async fn send(&mut self, bytes: &[u8]) -> Result<(), Failure> {
        HandlerSupervisor::write_request(&mut self.stdin, bytes).await
    }

    pub(super) async fn next_output(&mut self) -> WarmOutput {
        self.output.recv().await.unwrap_or(WarmOutput::Ended)
    }

    pub(super) fn take_stderr(&self) -> Drained {
        self.stderr.take()
    }

    pub(super) async fn cancel(mut self, request_id: &str, grace: Duration, deadline: Instant) {
        ProcessReaper::cancel_and_reap(
            &mut self.child,
            Some(&mut self.stdin),
            request_id,
            grace,
            deadline,
        )
        .await;
        self.reader.abort();
    }

    pub(super) async fn retire(mut self) {
        ProcessReaper::terminate(&mut self.child, Self::LABEL).await;
        self.reader.abort();
    }

    pub(super) async fn exit_report(mut self, deadline: Instant) -> (Option<ExitStatus>, Drained) {
        let waited = timeout_at(deadline, self.child.inner().wait()).await;
        let status = waited.ok().and_then(Result::ok);
        ProcessReaper::terminate(&mut self.child, Self::LABEL).await;
        self.reader.abort();
        let stderr = self.stderr.take_after_exit(deadline).await;
        (status, stderr)
    }

    async fn ready_or_retired(mut self) -> Option<Self> {
        if self.is_ready() {
            return Some(self);
        }
        self.retire().await;
        None
    }

    fn still_running(&mut self) -> bool {
        matches!(self.child.inner().try_wait(), Ok(None))
    }
}
