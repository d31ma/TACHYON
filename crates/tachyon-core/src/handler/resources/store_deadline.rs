use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use crate::storage::BusyClock;

pub(in crate::handler) struct StoreDeadline {
    per_call: AtomicBool,
    store_time: BusyClock,
    startup_used: Mutex<Duration>,
}

impl StoreDeadline {
    pub(in crate::handler) fn new() -> Self {
        Self {
            per_call: AtomicBool::new(false),
            store_time: BusyClock::new(),
            startup_used: Mutex::new(Duration::ZERO),
        }
    }

    pub(in crate::handler) fn use_per_call(&self) {
        self.per_call.store(true, Ordering::Release);
    }

    pub(in crate::handler) fn use_per_call_when(&self, asked: bool) {
        self.per_call.fetch_or(asked, Ordering::Release);
    }

    pub(in crate::handler) fn begin_call(&self, time_limit: Duration) -> Option<Instant> {
        self.store_time.begin();
        let budget = time_limit + *Self::locked(&self.startup_used);
        let left = budget.saturating_sub(self.store_time.spent_now());
        let per_call = self.per_call.load(Ordering::Acquire);
        (!per_call).then(|| Instant::now() + left)
    }

    pub(in crate::handler) fn end_call(&self, extended: Duration) {
        *Self::locked(&self.startup_used) += extended;
        self.store_time.end();
    }

    pub(in crate::handler) fn startup_left(&self, allowance: Duration) -> Duration {
        let per_call = self.per_call.load(Ordering::Acquire);
        let used = [*Self::locked(&self.startup_used), Duration::ZERO][usize::from(per_call)];
        allowance.saturating_sub(used)
    }

    fn locked<Value>(value: &Mutex<Value>) -> MutexGuard<'_, Value> {
        value.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::StoreDeadline;

    fn left_after(deadline: Option<Instant>) -> Duration {
        deadline.map_or(Duration::ZERO, |deadline| {
            deadline.saturating_duration_since(Instant::now())
        })
    }

    #[test]
    fn a_test_unit_asks_for_a_deadline_per_store_call() {
        let asked = StoreDeadline::new();
        asked.use_per_call_when(true);
        assert_eq!(asked.begin_call(Duration::from_secs(5)), None);
        let not_asked = StoreDeadline::new();
        not_asked.use_per_call_when(false);
        assert!(not_asked.begin_call(Duration::from_secs(5)).is_some());
    }

    #[test]
    fn time_outside_store_calls_does_not_count() {
        let deadline = StoreDeadline::new();
        deadline.begin_call(Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(100));
        deadline.end_call(Duration::ZERO);
        std::thread::sleep(Duration::from_millis(700));
        let left = left_after(deadline.begin_call(Duration::from_secs(2)));
        assert!(left > Duration::from_millis(1_600), "{left:?}");
        assert!(left <= Duration::from_millis(1_900), "{left:?}");
    }

    #[test]
    fn a_call_that_used_the_whole_budget_leaves_nothing_for_the_next() {
        let deadline = StoreDeadline::new();
        deadline.begin_call(Duration::from_millis(100));
        std::thread::sleep(Duration::from_millis(120));
        deadline.end_call(Duration::ZERO);
        let left = left_after(deadline.begin_call(Duration::from_millis(100)));
        assert_eq!(left, Duration::ZERO);
    }

    #[test]
    fn parallel_calls_are_counted_once() {
        let deadline = StoreDeadline::new();
        deadline.begin_call(Duration::from_secs(2));
        deadline.begin_call(Duration::from_secs(2));
        std::thread::sleep(Duration::from_millis(400));
        deadline.end_call(Duration::ZERO);
        deadline.end_call(Duration::ZERO);
        let left = left_after(deadline.begin_call(Duration::from_secs(2)));
        assert!(left > Duration::from_millis(1_400), "{left:?}");
        assert!(left <= Duration::from_millis(1_600), "{left:?}");
    }

    #[test]
    fn startup_time_is_given_back_to_the_budget() {
        let deadline = StoreDeadline::new();
        deadline.begin_call(Duration::from_secs(1));
        std::thread::sleep(Duration::from_millis(300));
        deadline.end_call(Duration::from_millis(300));
        let left = left_after(deadline.begin_call(Duration::from_secs(1)));
        assert!(left > Duration::from_millis(850), "{left:?}");
    }

    #[test]
    fn the_startup_allowance_is_shared_by_one_answer() {
        let deadline = StoreDeadline::new();
        let allowance = Duration::from_secs(20);
        assert_eq!(deadline.startup_left(allowance), allowance);
        deadline.end_call(Duration::from_secs(15));
        assert_eq!(deadline.startup_left(allowance), Duration::from_secs(5));
        deadline.end_call(Duration::from_secs(5));
        assert_eq!(deadline.startup_left(allowance), Duration::ZERO);
    }

    #[test]
    fn a_stream_call_gets_the_whole_startup_allowance() {
        let deadline = StoreDeadline::new();
        deadline.use_per_call();
        deadline.end_call(Duration::from_secs(15));
        let allowance = Duration::from_secs(20);
        assert_eq!(deadline.startup_left(allowance), allowance);
    }

    #[test]
    fn a_stream_or_socket_gives_each_store_call_its_own_deadline() {
        let deadline = StoreDeadline::new();
        deadline.use_per_call();
        assert_eq!(deadline.begin_call(Duration::from_secs(5)), None);
    }
}
