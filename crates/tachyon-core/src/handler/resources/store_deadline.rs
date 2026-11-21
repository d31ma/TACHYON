use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

pub(in crate::handler) struct StoreDeadline {
    per_call: AtomicBool,
    shared: OnceLock<Instant>,
}

impl StoreDeadline {
    pub(in crate::handler) fn new() -> Self {
        Self {
            per_call: AtomicBool::new(false),
            shared: OnceLock::new(),
        }
    }

    pub(in crate::handler) fn use_per_call(&self) {
        self.per_call.store(true, Ordering::Release);
    }

    pub(in crate::handler) fn use_per_call_when(&self, asked: bool) {
        self.per_call.fetch_or(asked, Ordering::Release);
    }

    pub(in crate::handler) fn for_call(&self, time_limit: Duration) -> Option<Instant> {
        let shared = *self.shared.get_or_init(|| Instant::now() + time_limit);
        let per_call = self.per_call.load(Ordering::Acquire);
        (!per_call).then_some(shared)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::StoreDeadline;

    #[test]
    fn every_store_call_of_one_answer_shares_the_first_calls_deadline() {
        let deadline = StoreDeadline::new();
        let first = deadline.for_call(Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(5));
        let later = deadline.for_call(Duration::from_secs(5));
        assert!(first.is_some());
        assert_eq!(first, later);
    }

    #[test]
    fn a_test_unit_asks_for_a_deadline_per_store_call() {
        let asked = StoreDeadline::new();
        asked.use_per_call_when(true);
        assert_eq!(asked.for_call(Duration::from_secs(5)), None);
        let not_asked = StoreDeadline::new();
        not_asked.use_per_call_when(false);
        assert!(not_asked.for_call(Duration::from_secs(5)).is_some());
    }

    #[test]
    fn a_stream_or_socket_gives_each_store_call_its_own_deadline() {
        let deadline = StoreDeadline::new();
        deadline.use_per_call();
        assert_eq!(deadline.for_call(Duration::from_secs(5)), None);
    }
}
