use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::time::Instant;

pub(super) struct EgressActivity {
    started: Instant,
    last: AtomicU64,
}

impl EgressActivity {
    pub(super) fn new() -> Self {
        Self {
            started: Instant::now(),
            last: AtomicU64::new(0),
        }
    }

    pub(super) fn mark(&self) {
        let elapsed = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.last.store(elapsed, Ordering::Relaxed);
    }

    pub(super) async fn until_idle(&self, idle: Duration) {
        self.wait_while_active(idle).await;
    }

    async fn wait_while_active(&self, idle: Duration) {
        while self.still_active(idle).await {}
    }

    async fn still_active(&self, idle: Duration) -> bool {
        let last = Duration::from_millis(self.last.load(Ordering::Relaxed));
        let deadline = self.started + last + idle;
        let active = Instant::now() < deadline;
        tokio::time::sleep_until(deadline).await;
        active
    }
}
