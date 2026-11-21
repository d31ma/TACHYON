use std::collections::HashSet;
use std::sync::{Mutex, MutexGuard, PoisonError};

use serde_json::Value;

use super::FyloError;

pub(super) struct StartupRequests {
    answered: Mutex<HashSet<String>>,
}

impl StartupRequests {
    const TARGET_FIELDS: [&'static str; 2] = ["collection", "topic"];

    pub(super) fn new() -> Self {
        Self {
            answered: Mutex::new(HashSet::new()),
        }
    }

    pub(super) fn restart(&self) {
        self.targets().clear();
    }

    pub(super) fn target_of(fields: &Value) -> String {
        let named = Self::TARGET_FIELDS
            .into_iter()
            .find_map(|field| fields[field].as_str());
        named.unwrap_or_default().to_owned()
    }

    pub(super) fn is_first(&self, target: &str) -> bool {
        !self.targets().contains(target)
    }

    pub(super) fn record(&self, target: String, answer: &Result<Value, FyloError>) {
        let started = Self::proves_started(answer);
        self.remember_when(started, target);
    }

    fn remember_when(&self, started: bool, target: String) {
        if started {
            self.targets().insert(target);
        }
    }

    fn proves_started(answer: &Result<Value, FyloError>) -> bool {
        answer
            .as_ref()
            .err()
            .is_none_or(|error| !error.broke_process() && !error.root_locked())
    }

    fn targets(&self) -> MutexGuard<'_, HashSet<String>> {
        self.answered.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::super::FyloError;
    use super::StartupRequests;

    fn failed(code: &str) -> Result<serde_json::Value, FyloError> {
        Err(FyloError {
            code: code.to_owned(),
        })
    }

    #[test]
    fn the_first_request_for_each_collection_after_a_start_is_a_startup_request() {
        let requests = StartupRequests::new();
        assert!(requests.is_first("people"));
        requests.record(String::from("people"), &Ok(json!("id")));
        assert!(!requests.is_first("people"));
        assert!(requests.is_first("local-parts"));
        requests.restart();
        assert!(requests.is_first("people"));
    }

    #[test]
    fn a_held_root_or_a_broken_pipe_does_not_count_as_started() {
        let requests = StartupRequests::new();
        requests.record(String::from("people"), &failed("EROOTLOCKED"));
        requests.record(String::from("people"), &failed("EPROTOCOL"));
        assert!(requests.is_first("people"));
        requests.record(String::from("people"), &failed("ENOTFOUND"));
        assert!(!requests.is_first("people"));
    }

    #[test]
    fn the_target_is_the_collection_or_the_queue_topic() {
        let document = json!({"op": "putData", "collection": "people"});
        let queue = json!({"op": "queuePublish", "topic": "outbox"});
        let sql = json!({"op": "executeSQL", "sql": "SELECT 1"});
        assert_eq!(StartupRequests::target_of(&document), "people");
        assert_eq!(StartupRequests::target_of(&queue), "outbox");
        assert_eq!(StartupRequests::target_of(&sql), "");
    }
}
