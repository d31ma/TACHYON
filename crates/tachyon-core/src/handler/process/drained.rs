use super::ProcessFailure;
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::frame::HandlerFrame;
use crate::sorted_json::SortedJson;
use serde_json::Value;
use std::process::ExitStatus;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::task::{JoinError, JoinHandle};
use tokio::time::error::Elapsed;
use tokio::time::{Instant, timeout_at};

#[derive(Debug)]
pub(in crate::handler) struct Drained {
    pub(in crate::handler) bytes: Vec<u8>,
    pub(in crate::handler) overflow: bool,
    pub(in crate::handler) read_error: bool,
}

impl Drained {
    const CHUNK_BYTES: usize = 8 * 1024;
    const MAX_START_CAPACITY: usize = 64 * 1024;
    const MAX_SIDEBAND_CHARACTERS: usize = 4_096;
    const KEPT_CONTROLS: [char; 2] = ['\n', '\t'];
    const KNOWN_RELAY_CATEGORIES: [&'static str; 5] =
        ["start", "timeout", "overflow", "exit", "protocol"];
    const EXCEPTION_EVENT: &'static str = "handler.cpp_exception";
    const STDERR_EVENT: &'static str = "handler.stderr";
    const STDERR_SETTING: &'static str = "TAC_HANDLER_STDERR";
    const MAX_TAIL_BYTES: usize = 16 * 1024;

    pub(in crate::handler) async fn read_all<R>(mut reader: R, limit: usize) -> Self
    where
        R: AsyncRead + Unpin,
    {
        let mut drained = Self {
            bytes: Vec::with_capacity(limit.min(Self::MAX_START_CAPACITY)),
            overflow: false,
            read_error: false,
        };
        drained.read_until_end(&mut reader, limit).await;
        drained
    }

    pub(in crate::handler) async fn settle_pair(
        mut stdout: JoinHandle<Self>,
        mut stderr: JoinHandle<Self>,
        deadline: Instant,
    ) -> Result<(Self, Self), Failure> {
        let stdout_result = timeout_at(deadline, &mut stdout).await;
        let stdout = Self::settled_stdout(stdout_result, stdout, &mut stderr).await?;
        let stderr_result = timeout_at(deadline, &mut stderr).await;
        let stderr = Self::settled_stderr(stderr_result, stderr).await?;
        Ok((stdout, stderr))
    }

    pub(super) async fn settle(
        mut task: JoinHandle<Self>,
        deadline: Instant,
    ) -> Result<Self, Failure> {
        let result = timeout_at(deadline, &mut task).await;
        Self::settled_diagnostics(result, task).await
    }

    pub(super) fn validate_output(
        status: ExitStatus,
        stdout: &Self,
        stderr: &Self,
    ) -> Result<(), Failure> {
        stdout.check_stdout_fits()?;
        stdout.check_stdout_complete()?;
        stderr.check_stderr_complete()?;
        stderr.check_exit_status(status)
    }

    pub(super) fn check_diagnostics(&self) -> Result<(), Failure> {
        self.check_stderr_complete()
    }

    pub(super) fn validate_stream_output(status: ExitStatus, stderr: &Self) -> Result<(), Failure> {
        let empty_stdout = Self {
            bytes: Vec::new(),
            overflow: false,
            read_error: false,
        };
        Self::validate_output(status, &empty_stdout, stderr)
    }

    pub(super) fn exit_failure(&self, status: ExitStatus) -> Failure {
        let suffix = Self::sideband_suffix(&Self::bounded_sideband(&self.bytes));
        Failure::one(diagnostic(
            2104,
            format!("Handler process exited unsuccessfully ({status}).{suffix}"),
            Some(String::from(
                "Fix the handler crash; a later invocation will start a clean process.",
            )),
            None,
        ))
    }

    pub(super) fn bounded_sideband(bytes: &[u8]) -> String {
        Self::visible_text(bytes, Self::MAX_SIDEBAND_CHARACTERS)
    }

    pub(super) fn stderr_event(&self, request_id: &str) -> Option<Value> {
        let text = Self::visible_text(&self.bytes, usize::MAX);
        (!text.is_empty()).then(|| {
            serde_json::json!({
                "event": Self::STDERR_EVENT,
                "request_id": request_id,
                "stderr": text,
                "truncated": self.overflow,
            })
        })
    }

    fn visible_text(bytes: &[u8], limit: usize) -> String {
        let text = String::from_utf8_lossy(bytes);
        let visible: String = text
            .chars()
            .filter(|character| Self::visible_character(*character))
            .take(limit)
            .collect();
        visible.trim().to_owned()
    }

    pub(super) fn surface_events(&self, request_id: &str) {
        self.events()
            .iter()
            .filter_map(|event| Self::relay_failure_category(event, request_id))
            .for_each(|category| Self::print_relay_failure(request_id, category));
        self.exceptions(request_id)
            .iter()
            .for_each(|line| eprintln!("{}", SortedJson::text(line)));
        self.report_stderr(request_id);
    }

    pub(super) fn report_settled(settled: &Result<Self, Failure>, request_id: &str) {
        Self::report_when_settled(settled.as_ref().ok(), request_id);
    }

    fn report_when_settled(settled: Option<&Self>, request_id: &str) {
        if let Some(stderr) = settled {
            stderr.report_stderr(request_id);
        }
    }

    pub(super) fn report_stderr(&self, request_id: &str) {
        let reported = crate::setting_switch::SettingSwitch::is_set(Self::STDERR_SETTING);
        self.print_stderr_event(reported, request_id);
    }

    fn print_stderr_event(&self, reported: bool, request_id: &str) {
        if reported && let Some(event) = self.stderr_event(request_id) {
            eprintln!("{}", SortedJson::text(&event));
        }
    }

    pub(super) fn reports_relay_failure(&self, request_id: &str) -> bool {
        self.events()
            .iter()
            .any(|event| Self::relay_failure_category(event, request_id).is_some())
    }

    pub(super) fn exceptions(&self, request_id: &str) -> Vec<Value> {
        self.events()
            .iter()
            .filter_map(|event| Self::exception(event, request_id))
            .collect()
    }

    fn events(&self) -> Vec<Value> {
        String::from_utf8_lossy(&self.bytes)
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .collect()
    }

    fn exception(event: &Value, request_id: &str) -> Option<Value> {
        let named = [
            event.get("event").and_then(Value::as_str) == Some(Self::EXCEPTION_EVENT),
            event.get("request_id").and_then(Value::as_str) == Some(request_id),
        ] == [true, true];
        let message = event
            .get("message")
            .and_then(Value::as_str)
            .filter(|_| named)?;
        Some(serde_json::json!({
            "event": Self::EXCEPTION_EVENT,
            "request_id": request_id,
            "message": Self::bounded_sideband(message.as_bytes()),
        }))
    }

    async fn read_until_end<R>(&mut self, reader: &mut R, limit: usize)
    where
        R: AsyncRead + Unpin,
    {
        let mut chunk = [0_u8; Self::CHUNK_BYTES];
        while self.read_chunk(reader, &mut chunk, limit).await {}
    }

    async fn read_chunk<R>(&mut self, reader: &mut R, chunk: &mut [u8], limit: usize) -> bool
    where
        R: AsyncRead + Unpin,
    {
        match reader.read(chunk).await {
            Ok(0) => false,
            Err(_) => {
                self.read_error = true;
                false
            }
            Ok(read) => {
                self.keep(&chunk[..read], limit);
                true
            }
        }
    }

    pub(super) fn keep(&mut self, data: &[u8], limit: usize) {
        let tail = Self::MAX_TAIL_BYTES.min(limit / 4);
        self.keep_within(data, limit - tail, tail);
    }

    fn keep_within(&mut self, data: &[u8], head: usize, tail: usize) {
        if self.overflow || self.bytes.len() + data.len() > head + tail {
            self.keep_head_and_tail(data, head, tail);
        } else {
            self.bytes.extend_from_slice(data);
        }
    }

    fn keep_head_and_tail(&mut self, data: &[u8], head: usize, tail: usize) {
        self.overflow = true;
        self.bytes.extend_from_slice(data);
        let length = self.bytes.len();
        let dropped_end = length.saturating_sub(tail).max(head);
        self.bytes.drain(head.min(length)..dropped_end);
    }

    async fn settled_stdout(
        result: Result<Result<Self, JoinError>, Elapsed>,
        stdout: JoinHandle<Self>,
        stderr: &mut JoinHandle<Self>,
    ) -> Result<Self, Failure> {
        match result {
            Err(_) => {
                stdout.abort();
                stderr.abort();
                let _stdout = stdout.await;
                let _stderr = stderr.await;
                Err(ProcessFailure::timed_out())
            }
            Ok(Err(_)) => {
                stderr.abort();
                let _stderr = stderr.await;
                Err(HandlerFrame::protocol_failure(
                    2101,
                    "Handler stdout drain task failed.",
                ))
            }
            Ok(Ok(stdout)) => Ok(stdout),
        }
    }

    async fn settled_stderr(
        result: Result<Result<Self, JoinError>, Elapsed>,
        stderr: JoinHandle<Self>,
    ) -> Result<Self, Failure> {
        match result {
            Err(_) => {
                stderr.abort();
                let _stderr = stderr.await;
                Err(ProcessFailure::timed_out())
            }
            Ok(Err(_)) => Err(HandlerFrame::protocol_failure(
                2101,
                "Handler stderr drain task failed.",
            )),
            Ok(Ok(stderr)) => Ok(stderr),
        }
    }

    async fn settled_diagnostics(
        result: Result<Result<Self, JoinError>, Elapsed>,
        task: JoinHandle<Self>,
    ) -> Result<Self, Failure> {
        match result {
            Ok(Ok(drained)) => Ok(drained),
            Ok(Err(_)) => Err(ProcessFailure::stream("Handler diagnostic reader failed.")),
            Err(_) => {
                task.abort();
                let _task = task.await;
                Err(ProcessFailure::timed_out())
            }
        }
    }

    fn check_stdout_fits(&self) -> Result<(), Failure> {
        if self.overflow {
            return Err(HandlerFrame::protocol_failure(
                2103,
                "Handler protocol stdout exceeds the 16 MiB frame limit.",
            ));
        }
        Ok(())
    }

    fn check_stdout_complete(&self) -> Result<(), Failure> {
        if self.read_error {
            return Err(HandlerFrame::protocol_failure(
                2102,
                "Cannot read the complete handler protocol stdout stream.",
            ));
        }
        Ok(())
    }

    fn check_stderr_complete(&self) -> Result<(), Failure> {
        if self.read_error {
            return Err(Failure::one(diagnostic(
                2107,
                "Cannot read the complete handler stderr stream.",
                Some(String::from("Retry with a healthy language runtime.")),
                None,
            )));
        }
        Ok(())
    }

    fn check_exit_status(&self, status: ExitStatus) -> Result<(), Failure> {
        if !status.success() {
            return Err(self.exit_failure(status));
        }
        Ok(())
    }

    fn sideband_suffix(sideband: &str) -> String {
        if sideband.is_empty() {
            return String::new();
        }
        format!(" Stderr: {sideband}")
    }

    fn visible_character(character: char) -> bool {
        !character.is_control() || Self::KEPT_CONTROLS.contains(&character)
    }

    fn relay_failure_category(event: &Value, request_id: &str) -> Option<&'static str> {
        let name = event.get("event").and_then(Value::as_str);
        let event_request_id = event.get("request_id").and_then(Value::as_str);
        if name != Some("handler.relay_failed") || event_request_id != Some(request_id) {
            return None;
        }
        let category = event.get("category").and_then(Value::as_str)?;
        Self::KNOWN_RELAY_CATEGORIES
            .into_iter()
            .find(|known| *known == category)
    }

    fn print_relay_failure(request_id: &str, category: &str) {
        eprintln!(
            "{}",
            SortedJson::text(&serde_json::json!({
                "event": "handler.relay_failed",
                "request_id": request_id,
                "category": category,
            }))
        );
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::Drained;
    use std::process::ExitStatus;

    fn stderr(text: &str, limit: usize) -> Drained {
        let mut drained = Drained {
            bytes: Vec::new(),
            overflow: false,
            read_error: false,
        };
        drained.keep(text.as_bytes(), limit);
        drained
    }

    #[test]
    fn stderr_past_its_limit_is_truncated_and_the_call_still_succeeds() {
        let stderr = stderr(&"x".repeat(70 * 1024), 64 * 1024);
        assert_eq!(stderr.bytes.len(), 64 * 1024);
        assert!(stderr.overflow);
        assert!(stderr.check_diagnostics().is_ok());
        assert!(Drained::validate_stream_output(ExitStatus::default(), &stderr).is_ok());
    }

    #[test]
    fn the_stderr_event_carries_the_kept_text_and_says_when_it_was_cut() {
        let event = stderr("first line\nsecond\u{7}", 1024)
            .stderr_event("0ABC")
            .expect("event");
        assert_eq!(event["event"], "handler.stderr");
        assert_eq!(event["request_id"], "0ABC");
        assert_eq!(event["stderr"], "first line\nsecond");
        assert_eq!(event["truncated"], false);
        let cut = stderr("abcdef", 3).stderr_event("0ABC").expect("cut event");
        assert_eq!(cut["stderr"], "abc");
        assert_eq!(cut["truncated"], true);
        assert!(stderr("", 3).stderr_event("0ABC").is_none());
    }

    #[test]
    fn an_overflowing_stderr_keeps_its_start_and_its_last_events() {
        let mut stderr = stderr(&"x".repeat(80 * 1024), 64 * 1024);
        let event =
            "{\"event\":\"handler.relay_failed\",\"request_id\":\"0ABC\",\"category\":\"exit\"}\n";
        stderr.keep(format!("\n{event}").as_bytes(), 64 * 1024);
        assert!(stderr.overflow);
        assert_eq!(stderr.bytes.len(), 64 * 1024);
        assert!(stderr.bytes.starts_with(b"xxxx"));
        assert!(stderr.reports_relay_failure("0ABC"));
    }
}
