use std::sync::Arc;
use std::time::Instant;

use serde_json::Value;

use super::store_call_line::StoreCallLine;
use crate::sorted_json::SortedJson;
use crate::storage::{ManagedStorage, StoreCall, StoreOperation};

pub(in crate::handler) struct UnfinishedStoreCall {
    request_id: String,
    summary: Value,
    call: Arc<StoreCall>,
    storage: Arc<ManagedStorage>,
    started: Instant,
    finished: bool,
}

impl UnfinishedStoreCall {
    const CANCELLED: &'static str = "cancelled";

    pub(in crate::handler) fn new(
        request_id: &str,
        command: &Value,
        call: &Arc<StoreCall>,
        storage: &Arc<ManagedStorage>,
    ) -> Self {
        Self {
            request_id: request_id.to_owned(),
            summary: StoreOperation::summary(command),
            call: Arc::clone(call),
            storage: Arc::clone(storage),
            started: Instant::now(),
            finished: false,
        }
    }

    pub(in crate::handler) fn finish(mut self) {
        self.finished = true;
    }

    fn report_cancelled(&self) {
        Self::print(self.cancelled_event());
    }

    fn cancelled_event(&self) -> Option<Value> {
        let line = StoreCallLine::new(
            &self.request_id,
            &self.summary,
            self.call.waited(),
            self.started.elapsed(),
        )
        .with_startup(self.call.started_up());
        line.event_with_code(self.storage.call_log(), Self::CANCELLED)
    }

    fn print(event: Option<Value>) {
        if let Some(event) = event {
            eprintln!("{}", SortedJson::text(&event));
        }
    }
}

impl Drop for UnfinishedStoreCall {
    fn drop(&mut self) {
        if !self.finished {
            self.report_cancelled();
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::sync::Arc;
    use std::time::Duration;

    use serde_json::json;

    use super::UnfinishedStoreCall;
    use crate::storage::ManagedStorage;

    #[tokio::test]
    async fn a_call_that_never_finished_reports_its_real_time_as_cancelled() {
        let settings = json!({"yon": {"datasource": {"driver": "sqlite", "url": "sqlite::memory:", "logcalls": true}}});
        let storage = Arc::new(ManagedStorage::connect(&settings).await.expect("storage"));
        let command =
            json!({"operation": "sql", "mode": "rows", "sql": "SELECT 1", "parameters": []});
        let (call, _deadline) = storage.prepare_call(&command, None, Duration::ZERO);
        let unfinished = UnfinishedStoreCall::new("request-15", &command, &call, &storage);
        tokio::time::sleep(Duration::from_millis(60)).await;
        let event = unfinished
            .cancelled_event()
            .expect("logcalls writes every call");
        assert_eq!(event["result"], json!("cancelled"));
        assert_eq!(event["operation"], json!("sql.rows"));
        assert!(
            event["call_ms"].as_u64().is_some_and(|took| took >= 60),
            "{event}"
        );
        assert!(!event.to_string().contains("SELECT"), "{event}");
        unfinished.finish();
    }
}
