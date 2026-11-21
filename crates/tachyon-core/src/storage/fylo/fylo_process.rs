use std::path::Path;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::future::OptionFuture;
use serde_json::Value;
use tokio::process::Child;

use super::FyloError;
use super::fylo_input::FyloInput;
use super::fylo_pipes::FyloPipes;
use super::interruption::Interruption;
use super::request_watch::RequestWatch;
use super::stderr_tail::StderrTail;

pub(super) struct FyloProcess {
    child: Child,
    pipes: FyloPipes,
    interruption: Option<Interruption>,
    stderr: Option<StderrTail>,
}

impl FyloProcess {
    pub(super) const EXIT_GRACE: Duration = Duration::from_secs(1);
    pub(super) const SHUTDOWN_GRACE: Duration = Duration::from_secs(2);
    const SHUTDOWN: &'static str = "shutdown";
    const UNKNOWN_OPERATION: &'static str = "unknown";

    pub(super) fn spawn(root: &Path) -> Result<Self, FyloError> {
        let mut child = tokio::process::Command::new("fylo")
            .args(["exec", "--loop", "--root"])
            .arg(root)
            .args(["--max-request-bytes", "1048576", "--max-response-bytes", "8388608"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| {
                FyloError::protocol(
                    "The fylo binary is not on PATH; install FYLO (fylo.del.ma) to use this datasource.",
                )
            })?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdin is unavailable."))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdout is unavailable."))?;
        let stderr = child
            .stderr
            .take()
            .map(|stderr| StderrTail::follow(stderr, root));
        Ok(Self {
            child,
            pipes: FyloPipes::new(stdin, stdout),
            interruption: None,
            stderr,
        })
    }

    pub(super) async fn handshake(&mut self, request: &Value) -> Result<Value, FyloError> {
        self.pipes.request(request).await
    }

    pub(super) async fn request(
        &mut self,
        operation: &str,
        request: &Value,
        startup: bool,
    ) -> Result<Value, FyloError> {
        let watch = RequestWatch::start(self.pipes.input(), &mut self.interruption, operation);
        let answer = self.pipes.request_waiting(request, startup).await;
        watch.finish(&answer);
        answer
    }

    pub(super) fn input(&self) -> Arc<FyloInput> {
        self.pipes.input()
    }

    pub(super) async fn close(mut self, closed: Instant) {
        self.interruption
            .get_or_insert_with(|| Interruption::new(Self::SHUTDOWN))
            .mark_stopping_at(closed);
        self.stop_within(Self::SHUTDOWN_GRACE).await;
    }

    pub(super) fn must_stop(&self) -> bool {
        self.interruption.is_some()
    }

    pub(super) async fn wait_for_exit(&mut self) {
        self.stop_within(Self::EXIT_GRACE).await;
    }

    async fn stop_within(&mut self, grace: Duration) {
        let started = Instant::now();
        self.pipes.input().close();
        let status = self.exit_within(self.grace_left(grace)).await;
        let ended_well = status.is_some_and(|status| status.success());
        self.kill_unless(status.is_some()).await;
        let waited = started.elapsed();
        self.interruption
            .iter()
            .for_each(|interruption| interruption.report_stopped(waited));
        self.report_errors_unless(ended_well).await;
    }

    async fn report_errors_unless(&mut self, ended_well: bool) {
        if ended_well {
            return;
        }
        let operation = self
            .interruption
            .as_ref()
            .map_or(Self::UNKNOWN_OPERATION, Interruption::operation);
        OptionFuture::from(self.stderr.as_mut().map(|tail| tail.report(operation))).await;
    }

    fn grace_left(&self, grace: Duration) -> Duration {
        let since_stop = self
            .interruption
            .as_ref()
            .map_or(Duration::ZERO, Interruption::time_since_stop);
        grace.saturating_sub(since_stop)
    }

    async fn exit_within(&mut self, grace: Duration) -> Option<ExitStatus> {
        let pipes = &mut self.pipes;
        let child = &mut self.child;
        let exit = async {
            pipes.drain().await;
            child.wait().await
        };
        let waited = tokio::time::timeout(grace, exit).await.ok();
        waited.and_then(Result::ok)
    }

    async fn kill_unless(&mut self, exited: bool) {
        if exited {
            return;
        }
        let state = Self::process_state(self.child.id());
        self.interruption
            .iter_mut()
            .for_each(|interruption| interruption.mark_killed(state.clone()));
        let _already_exited = self.child.start_kill();
        let _exit_status_is_not_needed = self.child.wait().await;
    }

    #[cfg(target_os = "linux")]
    fn process_state(id: Option<u32>) -> Option<String> {
        let stat = std::fs::read_to_string(format!("/proc/{}/stat", id?)).ok()?;
        let (_name, rest) = stat.rsplit_once(')')?;
        rest.split_whitespace().next().map(str::to_owned)
    }

    #[cfg(not(target_os = "linux"))]
    fn process_state(_id: Option<u32>) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::path::Path;
    use std::time::{Duration, Instant};

    use serde_json::{Value, json};

    use super::{FyloProcess, Interruption};

    fn fylo_runs_here() -> bool {
        let available = std::process::Command::new("fylo")
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success());
        assert!(
            available || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
            "fylo binary is required on this runner"
        );
        available
    }

    async fn started(root: &Path) -> FyloProcess {
        let mut process = FyloProcess::spawn(root).ok().expect("fylo starts");
        process
            .handshake(&json!({"op": "handshake"}))
            .await
            .ok()
            .expect("handshake");
        let create = json!({"op": "createCollection", "collection": "hd-events"});
        process
            .request("createCollection", &create, true)
            .await
            .ok()
            .expect("collection");
        process
    }

    async fn drop_a_write_before_its_answer(process: &mut FyloProcess) {
        let write = json!({"op": "storeSave", "collection": "hd-events", "entity": {"n": 1}});
        let request = process.request("storeSave", &write, false);
        tokio::select! {
            biased;
            _answered = request => panic!("fylo answered before the first poll ended"),
            () = std::future::ready(()) => {}
        }
    }

    fn stop_path(process: &FyloProcess) -> Option<&'static str> {
        process.interruption.as_ref().map(Interruption::stop_path)
    }

    async fn stored_documents(root: &Path) -> usize {
        let mut process = FyloProcess::spawn(root).ok().expect("second fylo");
        let count = json!({"op": "storeCount", "collection": "hd-events", "query": {"where": []}});
        let answer: Value = process
            .request("storeCount", &count, false)
            .await
            .ok()
            .expect("count");
        process.close(Instant::now()).await;
        answer.as_u64().and_then(|count| usize::try_from(count).ok()).expect("a count")
    }

    #[tokio::test]
    async fn closing_the_input_lets_fylo_finish_the_write_and_exit() {
        if !fylo_runs_here() {
            return;
        }
        let directory = tempfile::tempdir().expect("root");
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        let mut process = started(&root).await;
        drop_a_write_before_its_answer(&mut process).await;
        assert!(process.must_stop());
        let started = Instant::now();
        process.wait_for_exit().await;
        assert_eq!(stop_path(&process), Some("closed"));
        assert!(started.elapsed() < Duration::from_secs(1));
        assert_eq!(stored_documents(&root).await, 1);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_fylo_that_does_not_exit_is_killed_after_one_second() {
        if !fylo_runs_here() {
            return;
        }
        let directory = tempfile::tempdir().expect("root");
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        let mut process = started(&root).await;
        let process_id = process.child.id().expect("running fylo").to_string();
        let paused = std::process::Command::new("kill")
            .args(["-STOP", &process_id])
            .status()
            .expect("kill -STOP");
        assert!(paused.success());
        drop_a_write_before_its_answer(&mut process).await;
        let started = Instant::now();
        process.wait_for_exit().await;
        assert_eq!(stop_path(&process), Some("killed"));
        assert!(started.elapsed() >= Duration::from_millis(900));
    }
}
