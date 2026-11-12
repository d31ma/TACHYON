use crate::Failure;
use crate::sorted_json::SortedJson;
use crate::storage::ManagedStorage;
use serde_json::{Value, json};
use std::sync::Arc;
use tachyon_diagnostics::Diagnostic;

use super::store_deadline::StoreDeadline;

#[derive(Clone)]
pub(in crate::handler) struct InvocationStorage {
    storage: Arc<ManagedStorage>,
    request_id: String,
    deadline: Arc<StoreDeadline>,
}

impl InvocationStorage {
    const FAILED_EVENT: &'static str = "handler.storage_failed";

    pub(in crate::handler) fn new(storage: Arc<ManagedStorage>, request_id: String) -> Self {
        Self {
            storage,
            request_id,
            deadline: Arc::new(StoreDeadline::new()),
        }
    }

    pub(super) fn store_deadline(&self) -> Arc<StoreDeadline> {
        Arc::clone(&self.deadline)
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        let shared = self.deadline.for_call(self.storage.operation_timeout());
        let result = self.storage.execute_within(command, shared).await;
        self.report_failure(&result);
        result
    }

    fn report_failure(&self, result: &Result<Value, Failure>) {
        if let Err(failure) = result {
            eprintln!("{}", SortedJson::text(&self.failure_event(failure)));
        }
    }

    fn failure_event(&self, failure: &Failure) -> Value {
        let diagnostics: Vec<Value> = failure
            .diagnostics()
            .iter()
            .map(Self::diagnostic_fields)
            .collect();
        json!({
            "event": Self::FAILED_EVENT,
            "request_id": self.request_id,
            "diagnostics": diagnostics,
        })
    }

    fn diagnostic_fields(diagnostic: &Diagnostic) -> Value {
        json!({
            "code": diagnostic.code.to_string(),
            "message": diagnostic.message,
        })
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::InvocationStorage;
    use crate::Failure;
    use crate::failure::diagnostic;
    use crate::storage::ManagedStorage;
    use serde_json::json;
    use std::sync::Arc;

    async fn invocation_storage() -> InvocationStorage {
        let storage = ManagedStorage::connect(
            &json!({"yon":{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}}),
        )
        .await
        .expect("storage");
        InvocationStorage::new(Arc::new(storage), String::from("request-7"))
    }

    #[tokio::test]
    async fn the_failure_event_names_the_request_and_the_diagnostic() {
        let storage = invocation_storage().await;
        let failure = Failure::one(diagnostic(
            2160,
            "FYLO operation failed (EROOTLOCKED).",
            None,
            None,
        ));
        let event = storage.failure_event(&failure);
        assert_eq!(
            event,
            json!({
                "event": "handler.storage_failed",
                "request_id": "request-7",
                "diagnostics": [{"code": "TY2160", "message": "FYLO operation failed (EROOTLOCKED)."}],
            })
        );
    }

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

    async fn fylo_storage(root: &std::path::Path) -> ManagedStorage {
        let path = root.display().to_string().replace('\\', "/");
        let path = path.strip_prefix("//?/").unwrap_or(&path).to_owned();
        let url = format!("fylo:///{}", path.trim_start_matches('/'));
        let datasource = json!({"driver": "fylo", "url": url, "timeout": 1000});
        ManagedStorage::connect(&json!({"yon": {"datasource": datasource}}))
            .await
            .expect("fylo storage")
    }

    #[tokio::test]
    async fn store_calls_of_one_answer_share_one_deadline_while_the_root_is_held() {
        if !fylo_runs_here() {
            return;
        }
        let directory = tempfile::tempdir().expect("root");
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        let holder = fylo_storage(&root).await;
        let create = json!({"operation": "document", "command": "createCollection", "collection": "hd-events"});
        holder
            .execute(&create)
            .await
            .expect("the holder takes the root");
        let storage = InvocationStorage::new(
            Arc::new(fylo_storage(&root).await),
            String::from("request-8"),
        );
        let exists = json!({"operation": "document", "command": "collectionExists", "collection": "hd-events"});
        let started = std::time::Instant::now();
        for _ in 0..3 {
            let failure = storage
                .execute(&exists)
                .await
                .expect_err("held root")
                .to_string();
            assert!(
                failure.contains("deadline exceeded during FYLO inspectCollection"),
                "{failure}"
            );
        }
        let waited = started.elapsed();
        assert!(
            waited >= std::time::Duration::from_millis(900),
            "{waited:?}"
        );
        assert!(
            waited < std::time::Duration::from_millis(2_500),
            "{waited:?}"
        );
        holder.close().await;
    }

    #[tokio::test]
    async fn a_failed_command_still_answers_the_same_failure() {
        let storage = invocation_storage().await;
        let answer = storage.execute(&json!({"operation": "unknown"})).await;
        assert!(answer.is_err());
    }
}
