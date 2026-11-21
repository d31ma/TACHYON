use std::time::Duration;

use serde_json::{Value, json};

use crate::Failure;
use crate::storage::{StoreCallLog, StoreOperation};

pub(in crate::handler) struct StoreCallLine<'call> {
    request_id: &'call str,
    command: &'call Value,
    waited: Duration,
    started_up: Duration,
    took: Duration,
}

impl<'call> StoreCallLine<'call> {
    const SLOW_EVENT: &'static str = "handler.storage_slow";
    const CALL_EVENT: &'static str = "handler.storage_call";
    const SUCCESS: &'static str = "ok";

    pub(in crate::handler) fn new(
        request_id: &'call str,
        command: &'call Value,
        waited: Duration,
        took: Duration,
    ) -> Self {
        Self {
            request_id,
            command,
            waited: waited.min(took),
            started_up: Duration::ZERO,
            took,
        }
    }

    pub(in crate::handler) fn with_startup(self, started_up: Duration) -> Self {
        Self {
            started_up: started_up.min(self.waited),
            ..self
        }
    }

    pub(in crate::handler) fn event(
        &self,
        log: &StoreCallLog,
        result: &Result<Value, Failure>,
    ) -> Option<Value> {
        self.event_with_code(log, &Self::result_code(result))
    }

    pub(in crate::handler) fn event_with_code(
        &self,
        log: &StoreCallLog,
        code: &str,
    ) -> Option<Value> {
        let name = self.event_name(log)?;
        let mut event = self.fields(name, log, code);
        event["collection"] = json!(StoreOperation::collection(self.command));
        Some(event)
    }

    fn event_name(&self, log: &StoreCallLog) -> Option<&'static str> {
        if log.is_slow(self.took) {
            return Some(Self::SLOW_EVENT);
        }
        log.logs_every_call().then_some(Self::CALL_EVENT)
    }

    fn fields(&self, name: &str, log: &StoreCallLog, code: &str) -> Value {
        json!({
            "event": name,
            "request_id": self.request_id,
            "datasource": log.datasource_of(self.command),
            "operation": StoreOperation::kind(self.command),
            "wait_ms": self.waited.as_millis(),
            "startup_ms": self.started_up.as_millis(),
            "call_ms": self.took.saturating_sub(self.waited).as_millis(),
            "result": code,
        })
    }

    fn result_code(result: &Result<Value, Failure>) -> String {
        let first = result
            .as_ref()
            .err()
            .and_then(|failure| failure.diagnostics().first());
        first.map_or_else(
            || Self::SUCCESS.to_owned(),
            |diagnostic| diagnostic.code.to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::{Value, json};

    use super::StoreCallLine;
    use crate::Failure;
    use crate::failure::diagnostic;
    use crate::storage::StoreCallLog;

    fn log(settings: &Value) -> StoreCallLog {
        StoreCallLog::from_configuration(&json!({ "datasource": settings }))
            .unwrap_or_else(|error| panic!("{error}"))
    }

    fn insert() -> Value {
        json!({"operation": "document", "command": "insert", "collection": "people", "value": {"email": "a@example.com"}})
    }

    #[test]
    fn a_slow_call_names_its_wait_its_own_time_and_its_result() {
        let command = insert();
        let line = StoreCallLine::new(
            "request-1",
            &command,
            Duration::from_millis(1_200),
            Duration::from_millis(1_500),
        );
        let failure = Failure::one(diagnostic(
            2160,
            "FYLO operation failed (EROOTLOCKED).",
            None,
            None,
        ));
        let event = line.event(&log(&json!({"driver": "fylo"})), &Err(failure));
        assert_eq!(
            event,
            Some(json!({
                "event": "handler.storage_slow",
                "request_id": "request-1",
                "datasource": "fylo",
                "operation": "document.insert",
                "collection": null,
                "wait_ms": 1200,
                "startup_ms": 0,
                "call_ms": 300,
                "result": "TY2160",
            }))
        );
        assert!(
            !event
                .unwrap_or_default()
                .to_string()
                .contains("example.com")
        );
    }

    #[test]
    fn a_fast_call_writes_nothing_unless_every_call_is_logged() {
        let command = insert();
        let line = StoreCallLine::new(
            "request-2",
            &command,
            Duration::ZERO,
            Duration::from_millis(3),
        );
        let quiet = line.event(&log(&json!({"driver": "fylo"})), &Ok(json!({})));
        assert_eq!(quiet, None);
        let every = line.event(
            &log(&json!({"driver": "fylo", "logcalls": true})),
            &Ok(json!({})),
        );
        let every = every.unwrap_or_default();
        assert_eq!(every["event"], json!("handler.storage_call"));
        assert_eq!(every["result"], json!("ok"));
        assert_eq!(every["call_ms"], json!(3));
    }
}
