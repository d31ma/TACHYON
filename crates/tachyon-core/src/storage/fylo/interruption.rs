use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::sorted_json::SortedJson;

pub(super) struct Interruption {
    operation: String,
    killed: Option<Instant>,
}

impl Interruption {
    const STOPPED_EVENT: &'static str = "storage.fylo_process_stopped";

    pub(super) fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_owned(),
            killed: None,
        }
    }

    pub(super) fn mark_killed(&mut self) {
        self.killed.get_or_insert_with(Instant::now);
    }

    pub(super) fn report_stopped(&self) {
        eprintln!("{}", SortedJson::text(&self.stopped_event()));
    }

    fn time_since_kill(&self) -> Duration {
        self.killed
            .map_or(Duration::ZERO, |killed| killed.elapsed())
    }

    fn stopped_event(&self) -> Value {
        json!({
            "event": Self::STOPPED_EVENT,
            "operation": self.operation,
            "exit_ms": self.time_since_kill().as_millis(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::Interruption;

    #[test]
    fn the_stopped_event_names_the_operation_and_the_time_since_the_kill() {
        let mut interruption = Interruption::new("putData");
        interruption.mark_killed();
        std::thread::sleep(Duration::from_millis(20));
        let event = interruption.stopped_event();
        assert_eq!(event["event"], json!("storage.fylo_process_stopped"));
        assert_eq!(event["operation"], json!("putData"));
        assert!(
            event["exit_ms"].as_u64().is_some_and(|exit| exit >= 20),
            "{event}"
        );
    }

    #[test]
    fn the_kill_time_is_kept_from_the_first_kill() {
        let mut interruption = Interruption::new("getDoc");
        assert_eq!(interruption.time_since_kill(), Duration::ZERO);
        interruption.mark_killed();
        std::thread::sleep(Duration::from_millis(20));
        interruption.mark_killed();
        assert!(interruption.time_since_kill() >= Duration::from_millis(20));
    }
}
