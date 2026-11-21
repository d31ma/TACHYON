use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::sorted_json::SortedJson;

pub(super) struct Interruption {
    operation: String,
    stopping: Option<Instant>,
    killed: bool,
}

impl Interruption {
    const STOPPED_EVENT: &'static str = "storage.fylo_process_stopped";
    const STOP_PATHS: [&'static str; 2] = ["closed", "killed"];

    pub(super) fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_owned(),
            stopping: None,
            killed: false,
        }
    }

    pub(super) fn mark_stopping(&mut self) {
        self.stopping.get_or_insert_with(Instant::now);
    }

    pub(super) fn mark_stopping_at(&mut self, closed: Instant) {
        let earliest = self
            .stopping
            .map_or(closed, |stopping| stopping.min(closed));
        self.stopping = Some(earliest);
    }

    pub(super) fn mark_killed(&mut self) {
        self.killed = true;
    }

    pub(super) fn report_stopped(&self) {
        eprintln!("{}", SortedJson::text(&self.stopped_event()));
    }

    pub(super) fn operation(&self) -> &str {
        &self.operation
    }

    pub(super) fn time_since_stop(&self) -> Duration {
        self.stopping
            .map_or(Duration::ZERO, |stopping| stopping.elapsed())
    }

    pub(super) fn stop_path(&self) -> &'static str {
        Self::STOP_PATHS[usize::from(self.killed)]
    }

    fn stopped_event(&self) -> Value {
        json!({
            "event": Self::STOPPED_EVENT,
            "operation": self.operation,
            "exit_ms": self.time_since_stop().as_millis(),
            "stop": self.stop_path(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use serde_json::json;

    use super::Interruption;

    #[test]
    fn the_stopped_event_names_the_operation_the_time_since_the_stop_and_the_path() {
        let mut interruption = Interruption::new("putData");
        interruption.mark_stopping();
        std::thread::sleep(Duration::from_millis(20));
        let event = interruption.stopped_event();
        assert_eq!(event["event"], json!("storage.fylo_process_stopped"));
        assert_eq!(event["operation"], json!("putData"));
        assert_eq!(event["stop"], json!("closed"));
        assert!(
            event["exit_ms"].as_u64().is_some_and(|exit| exit >= 20),
            "{event}"
        );
    }

    #[test]
    fn a_process_that_had_to_be_killed_says_so() {
        let mut interruption = Interruption::new("putData");
        interruption.mark_stopping();
        interruption.mark_killed();
        assert_eq!(interruption.stopped_event()["stop"], json!("killed"));
    }

    #[test]
    fn the_stop_time_is_kept_from_the_first_stop() {
        let mut interruption = Interruption::new("getDoc");
        assert_eq!(interruption.time_since_stop(), Duration::ZERO);
        interruption.mark_stopping();
        std::thread::sleep(Duration::from_millis(20));
        interruption.mark_stopping();
        assert!(interruption.time_since_stop() >= Duration::from_millis(20));
    }
}
