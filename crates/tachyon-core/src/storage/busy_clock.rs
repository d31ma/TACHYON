use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

pub(crate) struct BusyClock {
    spent: Mutex<Duration>,
    running: Mutex<usize>,
    running_since: Mutex<Instant>,
}

impl BusyClock {
    pub(crate) fn new() -> Self {
        Self {
            spent: Mutex::new(Duration::ZERO),
            running: Mutex::new(0),
            running_since: Mutex::new(Instant::now()),
        }
    }

    pub(crate) fn begin(&self) {
        let mut running = Self::locked(&self.running);
        self.start_when_idle(*running);
        *running += 1;
    }

    pub(crate) fn end(&self) {
        let mut running = Self::locked(&self.running);
        *running = running.saturating_sub(1);
        self.stop_when_idle(*running);
    }

    pub(crate) fn spent_now(&self) -> Duration {
        let running = *Self::locked(&self.running);
        let open = [Duration::ZERO, Self::locked(&self.running_since).elapsed()];
        *Self::locked(&self.spent) + open[usize::from(running > 0)]
    }

    pub(crate) fn is_running(&self) -> bool {
        *Self::locked(&self.running) > 0
    }

    fn start_when_idle(&self, running: usize) {
        if running == 0 {
            *Self::locked(&self.running_since) = Instant::now();
        }
    }

    fn stop_when_idle(&self, running: usize) {
        if running == 0 {
            *Self::locked(&self.spent) += Self::locked(&self.running_since).elapsed();
        }
    }

    fn locked<Value>(value: &Mutex<Value>) -> MutexGuard<'_, Value> {
        value.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::BusyClock;

    #[test]
    fn time_counts_only_while_something_runs() {
        let clock = BusyClock::new();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(clock.spent_now(), Duration::ZERO);
        clock.begin();
        std::thread::sleep(Duration::from_millis(30));
        assert!(clock.is_running());
        assert!(clock.spent_now() >= Duration::from_millis(30));
        clock.end();
        let after = clock.spent_now();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(clock.spent_now(), after);
        assert!(!clock.is_running());
    }

    #[test]
    fn overlapping_spans_count_once() {
        let clock = BusyClock::new();
        clock.begin();
        clock.begin();
        std::thread::sleep(Duration::from_millis(300));
        clock.end();
        assert!(clock.is_running());
        clock.end();
        let spent = clock.spent_now();
        assert!(spent >= Duration::from_millis(300));
        assert!(spent < Duration::from_millis(550), "{spent:?}");
    }
}
