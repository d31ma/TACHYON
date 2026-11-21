use std::time::Duration;

use tokio::sync::watch;

use super::busy_clock::BusyClock;

pub(crate) struct StartupActivity {
    clock: BusyClock,
    changes: watch::Sender<()>,
}

impl StartupActivity {
    pub(super) fn new() -> Self {
        Self {
            clock: BusyClock::new(),
            changes: watch::Sender::new(()),
        }
    }

    pub(super) fn begin(&self) {
        self.clock.begin();
        self.changes.send_replace(());
    }

    pub(super) fn end(&self) {
        self.clock.end();
        self.changes.send_replace(());
    }

    pub(super) fn spent_now(&self) -> Duration {
        self.clock.spent_now()
    }

    pub(super) fn is_running(&self) -> bool {
        self.clock.is_running()
    }

    pub(super) fn subscribe(&self) -> watch::Receiver<()> {
        self.changes.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::StartupActivity;

    #[tokio::test]
    async fn every_begin_and_end_is_announced() {
        let activity = StartupActivity::new();
        let mut changes = activity.subscribe();
        activity.begin();
        assert!(changes.has_changed().unwrap_or_default());
        changes.mark_unchanged();
        activity.end();
        assert!(changes.has_changed().unwrap_or_default());
        assert!(!activity.is_running());
    }
}
