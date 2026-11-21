use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

pub(in crate::handler) struct StoreDeadline {
    per_call: AtomicBool,
    shared: Mutex<Option<Instant>>,
    startup_used: Mutex<Duration>,
}

impl StoreDeadline {
    pub(in crate::handler) fn new() -> Self {
        Self {
            per_call: AtomicBool::new(false),
            shared: Mutex::new(None),
            startup_used: Mutex::new(Duration::ZERO),
        }
    }

    pub(in crate::handler) fn use_per_call(&self) {
        self.per_call.store(true, Ordering::Release);
    }

    pub(in crate::handler) fn for_call(&self, time_limit: Duration) -> Option<Instant> {
        let shared = *self
            .slot()
            .get_or_insert_with(|| Instant::now() + time_limit);
        let per_call = self.per_call.load(Ordering::Acquire);
        (!per_call).then_some(shared)
    }

    pub(in crate::handler) fn startup_left(&self, allowance: Duration) -> Duration {
        let per_call = self.per_call.load(Ordering::Acquire);
        let used = [*Self::locked(&self.startup_used), Duration::ZERO][usize::from(per_call)];
        allowance.saturating_sub(used)
    }

    pub(in crate::handler) fn extend(&self, by: Duration) {
        *Self::locked(&self.startup_used) += by;
        self.slot().iter_mut().for_each(|deadline| *deadline += by);
    }

    fn slot(&self) -> MutexGuard<'_, Option<Instant>> {
        Self::locked(&self.shared)
    }

    fn locked<Value>(value: &Mutex<Value>) -> MutexGuard<'_, Value> {
        value.lock().unwrap_or_else(PoisonError::into_inner)
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
    fn startup_time_moves_the_shared_deadline() {
        let deadline = StoreDeadline::new();
        let first = deadline.for_call(Duration::from_secs(5));
        deadline.extend(Duration::from_secs(3));
        let later = deadline.for_call(Duration::from_secs(5));
        assert_eq!(later, first.map(|first| first + Duration::from_secs(3)));
    }

    #[test]
    fn the_startup_allowance_is_shared_by_one_answer() {
        let deadline = StoreDeadline::new();
        let allowance = Duration::from_secs(20);
        assert_eq!(deadline.startup_left(allowance), allowance);
        deadline.extend(Duration::from_secs(15));
        assert_eq!(deadline.startup_left(allowance), Duration::from_secs(5));
        deadline.extend(Duration::from_secs(5));
        assert_eq!(deadline.startup_left(allowance), Duration::ZERO);
    }

    #[test]
    fn a_stream_call_gets_the_whole_startup_allowance() {
        let deadline = StoreDeadline::new();
        deadline.use_per_call();
        deadline.extend(Duration::from_secs(15));
        let allowance = Duration::from_secs(20);
        assert_eq!(deadline.startup_left(allowance), allowance);
    }

    #[test]
    fn a_stream_or_socket_gives_each_store_call_its_own_deadline() {
        let deadline = StoreDeadline::new();
        deadline.use_per_call();
        assert_eq!(deadline.for_call(Duration::from_secs(5)), None);
    }
}
