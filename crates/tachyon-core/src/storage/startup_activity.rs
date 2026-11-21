use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use tokio::sync::watch;

pub(crate) struct StartupActivity {
    spent: Mutex<Duration>,
    running: Mutex<usize>,
    running_since: Mutex<Instant>,
    changes: watch::Sender<()>,
}

impl StartupActivity {
    pub(super) fn new() -> Self {
        Self {
            spent: Mutex::new(Duration::ZERO),
            running: Mutex::new(0),
            running_since: Mutex::new(Instant::now()),
            changes: watch::Sender::new(()),
        }
    }

    pub(super) fn begin(&self) {
        let mut running = Self::locked(&self.running);
        self.start_clock_when_idle(*running);
        *running += 1;
        drop(running);
        self.changes.send_replace(());
    }

    pub(super) fn end(&self) {
        let mut running = Self::locked(&self.running);
        *running = running.saturating_sub(1);
        self.stop_clock_when_idle(*running);
        drop(running);
        self.changes.send_replace(());
    }

    pub(super) fn spent_now(&self) -> Duration {
        let running = *Self::locked(&self.running);
        let open = [Duration::ZERO, Self::locked(&self.running_since).elapsed()];
        *Self::locked(&self.spent) + open[usize::from(running > 0)]
    }

    pub(super) fn is_running(&self) -> bool {
        *Self::locked(&self.running) > 0
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<()> {
        self.changes.subscribe()
    }

    fn start_clock_when_idle(&self, running: usize) {
        if running == 0 {
            *Self::locked(&self.running_since) = Instant::now();
        }
    }

    fn stop_clock_when_idle(&self, running: usize) {
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

    use super::StartupActivity;

    #[test]
    fn spent_time_counts_only_while_a_startup_runs() {
        let activity = StartupActivity::new();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(activity.spent_now(), Duration::ZERO);
        activity.begin();
        std::thread::sleep(Duration::from_millis(30));
        assert!(activity.is_running());
        assert!(activity.spent_now() >= Duration::from_millis(30));
        activity.end();
        let after = activity.spent_now();
        std::thread::sleep(Duration::from_millis(30));
        assert_eq!(activity.spent_now(), after);
        assert!(!activity.is_running());
    }

    #[test]
    fn overlapping_startups_count_once() {
        let activity = StartupActivity::new();
        activity.begin();
        activity.begin();
        std::thread::sleep(Duration::from_millis(100));
        activity.end();
        assert!(activity.is_running());
        activity.end();
        let spent = activity.spent_now();
        assert!(spent >= Duration::from_millis(100));
        assert!(spent < Duration::from_millis(190), "{spent:?}");
    }
}
