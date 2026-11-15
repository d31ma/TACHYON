use std::ffi::OsString;
use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Child;

use super::FyloError;
use super::frame_answer::FrameAnswer;
use super::fylo_root::FyloRoot;
use super::network_root::NetworkRoot;
use super::operation_deadline::OperationDeadline;
use crate::Failure;
use crate::sorted_json::SortedJson;
use crate::storage::failure;

pub(super) struct CallLease<'root> {
    root: &'root FyloRoot,
}

impl<'root> CallLease<'root> {
    const CLEANUP_RESERVE: Duration = Duration::from_millis(200);
    const ANSWER_MARGIN: Duration = Duration::from_millis(100);
    const MAX_REQUEST_BYTES: usize = 1024 * 1024;
    const MAX_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
    const DOCUMENT_INSERT: &'static str = "putData";
    const DEADLINE_CODE: &'static str = "EDEADLINE";
    const NO_FRAME_CODE: &'static str = "ENOFRAME";

    pub(super) fn new(root: &'root FyloRoot) -> Self {
        Self { root }
    }

    pub(super) async fn prepare(&self) -> Result<(), Failure> {
        NetworkRoot::refuse(self.root.path())?;
        let handshake = SortedJson::text(&json!({ "op": "handshake" }));
        let identity = self.run(&handshake, self.handshake_arguments()).await;
        Self::invocation_mode_confirmed(identity)
    }

    pub(super) async fn answer(&self, operation: &str, fields: Value) -> Result<Value, FyloError> {
        let request = Self::request_line(operation, fields)?;
        let deadline = Self::process_deadline(OperationDeadline::time_left())?;
        self.run(&request, self.invocation_arguments(deadline))
            .await
    }

    fn request_line(operation: &str, mut fields: Value) -> Result<String, FyloError> {
        fields["op"] = json!(operation);
        let request = Self::with_document_id(fields);
        Self::bounded(SortedJson::text(&request))
    }

    fn with_document_id(mut request: Value) -> Value {
        if request["op"] == Self::DOCUMENT_INSERT {
            request["id"] = json!(crate::ttid::generate());
        }
        request
    }

    fn bounded(line: String) -> Result<String, FyloError> {
        if line.len() > Self::MAX_REQUEST_BYTES {
            return Err(FyloError::protocol("FYLO request exceeds 1 MiB."));
        }
        Ok(line)
    }

    fn process_deadline(time_left: Duration) -> Result<Duration, FyloError> {
        let deadline = time_left.saturating_sub(Self::ANSWER_MARGIN);
        if deadline <= Self::CLEANUP_RESERVE {
            return Err(FyloError::coded(Self::DEADLINE_CODE));
        }
        Ok(deadline)
    }

    async fn run(&self, request: &str, arguments: Vec<OsString>) -> Result<Value, FyloError> {
        let mut child = Self::spawn(arguments)?;
        Self::send(&mut child, request).await?;
        let output = Self::read_output(&mut child).await?;
        child
            .wait()
            .await
            .map_err(|_| FyloError::protocol("FYLO invocation did not exit."))?;
        let line = Self::frame_line(Self::bounded_output(&output)?)?;
        FrameAnswer::from_line(line)
    }

    fn handshake_arguments(&self) -> Vec<OsString> {
        let mut arguments: Vec<OsString> = ["exec", "--request", "-"].map(OsString::from).to_vec();
        arguments.extend(self.root.arguments());
        arguments
    }

    fn invocation_arguments(&self, deadline: Duration) -> Vec<OsString> {
        let mut arguments = self.handshake_arguments();
        arguments.insert(1, OsString::from("--invocation"));
        let limits = [
            ("--deadline-ms", deadline.as_millis().to_string()),
            (
                "--cleanup-reserve-ms",
                Self::CLEANUP_RESERVE.as_millis().to_string(),
            ),
            ("--max-request-bytes", Self::MAX_REQUEST_BYTES.to_string()),
            ("--max-response-bytes", Self::MAX_RESPONSE_BYTES.to_string()),
        ];
        for (option, value) in limits {
            arguments.extend([OsString::from(option), OsString::from(value)]);
        }
        arguments
    }

    fn spawn(arguments: Vec<OsString>) -> Result<Child, FyloError> {
        tokio::process::Command::new("fylo")
            .args(arguments)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| FyloError::protocol("The fylo binary is not on PATH."))
    }

    async fn send(child: &mut Child, request: &str) -> Result<(), FyloError> {
        let mut input = child
            .stdin
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdin is unavailable."))?;
        input
            .write_all(request.as_bytes())
            .await
            .map_err(|_| FyloError::protocol("FYLO invocation did not read its request."))
    }

    async fn read_output(child: &mut Child) -> Result<Vec<u8>, FyloError> {
        let output = child
            .stdout
            .take()
            .ok_or_else(|| FyloError::protocol("FYLO stdout is unavailable."))?;
        let mut bytes = Vec::new();
        let limit = Self::MAX_RESPONSE_BYTES as u64 + 1;
        output
            .take(limit)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| FyloError::protocol("FYLO invocation closed its output."))?;
        Ok(bytes)
    }

    fn bounded_output(output: &[u8]) -> Result<&[u8], FyloError> {
        if output.len() > Self::MAX_RESPONSE_BYTES {
            return Err(FyloError::protocol("FYLO response exceeds 8 MiB."));
        }
        Ok(output)
    }

    fn frame_line(output: &[u8]) -> Result<&str, FyloError> {
        let line = std::str::from_utf8(output)
            .map_err(|_| FyloError::protocol("FYLO returned a malformed frame."))?;
        if line.trim().is_empty() {
            return Err(FyloError::coded(Self::NO_FRAME_CODE));
        }
        Ok(line)
    }

    fn invocation_mode_confirmed(identity: Result<Value, FyloError>) -> Result<(), Failure> {
        let capability = identity
            .map(|identity| identity["capabilities"]["invocation"].clone())
            .unwrap_or_default();
        Self::usable_capability(&capability)
    }

    fn usable_capability(capability: &Value) -> Result<(), Failure> {
        let found = [
            capability["version"].clone(),
            capability["waitsForRoot"].clone(),
            capability["createRequiresId"].clone(),
        ];
        if found != [json!(1), json!(true), json!(true)] {
            return Err(failure(
                "FYLO lease \"call\" needs a fylo whose handshake offers capabilities.invocation version 1 (FYLO ADR 0028); upgrade FYLO or use lease \"server\".",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::CallLease;
    use crate::storage::fylo::fylo_root::FyloRoot;
    use crate::storage::fylo::system_disk::SystemDisk;

    #[test]
    fn an_insert_gets_an_identifier_made_by_tachyon() {
        let line = CallLease::request_line("putData", json!({"collection": "notes"}));
        let request: serde_json::Value =
            serde_json::from_str(&line.unwrap_or_default()).unwrap_or_default();
        let id = request["id"].as_str().unwrap_or_default();
        assert!(crate::ttid::created_at_milliseconds(id).is_some_and(|time| time > 0));
    }

    #[test]
    fn other_operations_keep_their_fields() {
        let line = CallLease::request_line("getDoc", json!({"collection": "notes", "id": "A"}));
        let request: serde_json::Value =
            serde_json::from_str(&line.unwrap_or_default()).unwrap_or_default();
        assert_eq!(
            request,
            json!({"collection": "notes", "id": "A", "op": "getDoc"})
        );
    }

    #[test]
    fn a_request_over_one_mebibyte_is_refused_before_fylo_starts() {
        let text = "x".repeat(1024 * 1024);
        let refused = CallLease::request_line("putData", json!({"data": {"text": text}}));
        assert_eq!(
            refused.err().map(|error| error.code),
            Some("EPROTOCOL".to_owned())
        );
    }

    #[test]
    fn the_process_deadline_keeps_a_margin_to_read_the_answer() {
        let deadline = CallLease::process_deadline(Duration::from_secs(5)).ok();
        assert_eq!(deadline, Some(Duration::from_millis(4_900)));
    }

    #[test]
    fn time_left_inside_the_cleanup_reserve_fails_before_fylo_starts() {
        let refused = CallLease::process_deadline(Duration::from_millis(300));
        assert_eq!(
            refused.err().map(|error| error.code),
            Some("EDEADLINE".to_owned())
        );
    }

    #[test]
    fn an_empty_answer_is_a_missing_frame() {
        let refused = CallLease::frame_line(b"\n");
        assert_eq!(
            refused.err().map(|error| error.code),
            Some("ENOFRAME".to_owned())
        );
    }

    #[test]
    fn a_fylo_without_the_invocation_capability_is_refused() {
        let old = Ok(json!({"protocolVersion": 1, "capabilities": {"handshake": true}}));
        assert!(CallLease::invocation_mode_confirmed(old).is_err());
        let wasm = Ok(json!({"capabilities": {"invocation": false}}));
        assert!(CallLease::invocation_mode_confirmed(wasm).is_err());
        let failed = Err(super::FyloError::coded("ENOFRAME"));
        assert!(CallLease::invocation_mode_confirmed(failed).is_err());
    }

    #[test]
    fn a_fylo_that_offers_invocation_version_one_is_used() {
        let identity = Ok(json!({"capabilities": {"invocation": {
            "version": 1,
            "command": "exec --invocation",
            "createRequiresId": true,
            "waitsForRoot": true
        }}}));
        assert!(CallLease::invocation_mode_confirmed(identity).is_ok());
    }

    #[test]
    fn the_invocation_command_names_its_budget_and_limits() {
        let root = FyloRoot::new(std::path::PathBuf::from("/data/root"), SystemDisk::Allowed);
        let arguments = CallLease::new(&root).invocation_arguments(Duration::from_millis(4_900));
        let text: Vec<String> = arguments
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            text,
            [
                "exec",
                "--invocation",
                "--request",
                "-",
                "--root",
                "/data/root",
                "--allow-system-disk",
                "--deadline-ms",
                "4900",
                "--cleanup-reserve-ms",
                "200",
                "--max-request-bytes",
                "1048576",
                "--max-response-bytes",
                "8388608",
            ]
        );
    }
}
