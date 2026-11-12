use crate::Failure;
use crate::sorted_json::SortedJson;
use crate::storage::ManagedStorage;
use serde_json::{Value, json};
use std::sync::Arc;
use tachyon_diagnostics::Diagnostic;

#[derive(Clone)]
pub(in crate::handler) struct InvocationStorage {
    storage: Arc<ManagedStorage>,
    request_id: String,
}

impl InvocationStorage {
    const FAILED_EVENT: &'static str = "handler.storage_failed";

    pub(in crate::handler) fn new(storage: Arc<ManagedStorage>, request_id: String) -> Self {
        Self {
            storage,
            request_id,
        }
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        let result = self.storage.execute(command).await;
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

    #[tokio::test]
    async fn a_failed_command_still_answers_the_same_failure() {
        let storage = invocation_storage().await;
        let answer = storage.execute(&json!({"operation": "unknown"})).await;
        assert!(answer.is_err());
    }
}
