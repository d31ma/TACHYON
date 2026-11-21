use crate::Failure;
use crate::sorted_json::SortedJson;
use crate::storage::ManagedStorage;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tachyon_diagnostics::Diagnostic;

use super::store_call_line::StoreCallLine;
use super::store_deadline::StoreDeadline;
use crate::storage::StoreCall;

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
        let startup_left = self.deadline.startup_left(self.storage.startup_allowance());
        let started = Instant::now();
        let measured = self.storage.execute_measured(command, shared, startup_left);
        let (result, call) = Box::pin(measured).await;
        self.deadline.extend(call.extended());
        self.report_failure(&result);
        self.report_time(command, &call, started.elapsed(), &result);
        result
    }

    fn report_time(
        &self,
        command: &Value,
        call: &StoreCall,
        took: Duration,
        result: &Result<Value, Failure>,
    ) {
        let line = StoreCallLine::new(&self.request_id, command, call.waited(), took)
            .with_startup(call.started_up());
        if let Some(event) = line.event(self.storage.call_log(), result) {
            eprintln!("{}", SortedJson::text(&event));
        }
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
        fylo_storage_with(root, 20_000).await
    }

    async fn fylo_storage_with(root: &std::path::Path, startup: u64) -> ManagedStorage {
        let path = root.display().to_string().replace('\\', "/");
        let path = path.strip_prefix("//?/").unwrap_or(&path).to_owned();
        let url = format!("fylo:///{}", path.trim_start_matches('/'));
        let datasource = json!({"driver": "fylo", "url": url, "timeout": 1000, "startup": startup});
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

    #[cfg(unix)]
    fn pause_fylo_of(root: &std::path::Path) -> std::thread::JoinHandle<()> {
        let pattern = format!("fylo exec --loop --root {}", root.display());
        let found = std::process::Command::new("pgrep")
            .args(["-f", &pattern])
            .output()
            .expect("pgrep");
        let process_id = String::from_utf8_lossy(&found.stdout).trim().to_owned();
        let kill = |signal: &str| {
            std::process::Command::new("kill")
                .args([signal, process_id.as_str()])
                .status()
                .expect("kill")
        };
        assert!(kill("-STOP").success());
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1_500));
            let resumed = std::process::Command::new("kill")
                .args(["-CONT", &process_id])
                .status()
                .expect("kill -CONT");
            assert!(resumed.success());
        })
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn startup_time_does_not_use_up_the_deadline_of_the_answer() {
        if !fylo_runs_here() {
            return;
        }
        let directory = tempfile::tempdir().expect("root");
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        let storage = InvocationStorage::new(
            Arc::new(fylo_storage(&root).await),
            String::from("request-9"),
        );
        let resumed = pause_fylo_of(&root);
        let create = json!({"operation": "document", "command": "createCollection", "collection": "hd-events"});
        let created = storage.execute(&create).await;
        resumed.join().expect("resume thread");
        created.expect("the start-up request answers after the pause");
        let exists = json!({"operation": "document", "command": "collectionExists", "collection": "hd-events"});
        let answer = storage.execute(&exists).await;
        assert_eq!(answer.expect("the same answer may still call"), json!(true));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn one_answer_gets_at_most_the_startup_setting_of_extra_time() {
        if !fylo_runs_here() {
            return;
        }
        let directory = tempfile::tempdir().expect("root");
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        let storage = InvocationStorage::new(
            Arc::new(fylo_storage_with(&root, 1_000).await),
            String::from("request-10"),
        );
        let create = |name: &str| json!({"operation": "document", "command": "createCollection", "collection": name});
        let resumed = pause_fylo_of(&root);
        let first = storage.execute(&create("hd-first")).await;
        resumed.join().expect("resume thread");
        first.expect("the first start-up request uses the allowance");
        let resumed = pause_fylo_of(&root);
        let second = storage.execute(&create("hd-second")).await;
        resumed.join().expect("resume thread");
        let failure = second.expect_err("no allowance is left").to_string();
        assert!(
            failure.contains("deadline exceeded during FYLO createCollection"),
            "{failure}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn an_answer_that_waits_behind_another_answers_startup_keeps_its_time() {
        if !fylo_runs_here() {
            return;
        }
        let directory = tempfile::tempdir().expect("root");
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        let shared = Arc::new(fylo_storage(&root).await);
        let old =
            json!({"operation": "document", "command": "createCollection", "collection": "hd-old"});
        shared.execute(&old).await.expect("old collection");
        let first = InvocationStorage::new(Arc::clone(&shared), String::from("request-11"));
        let second = InvocationStorage::new(Arc::clone(&shared), String::from("request-12"));
        let resumed = pause_fylo_of(&root);
        let new =
            json!({"operation": "document", "command": "createCollection", "collection": "hd-new"});
        let exists =
            json!({"operation": "document", "command": "collectionExists", "collection": "hd-old"});
        let waits = async {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            second.execute(&exists).await
        };
        let (created, found) = tokio::join!(first.execute(&new), waits);
        resumed.join().expect("resume thread");
        created.expect("the start-up request answers");
        assert_eq!(
            found.expect("the waiting answer keeps its time"),
            json!(true)
        );
    }

    #[tokio::test]
    async fn a_failed_command_still_answers_the_same_failure() {
        let storage = invocation_storage().await;
        let answer = storage.execute(&json!({"operation": "unknown"})).await;
        assert!(answer.is_err());
    }
}
