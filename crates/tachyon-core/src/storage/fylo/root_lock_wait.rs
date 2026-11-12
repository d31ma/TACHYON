use std::time::Duration;

use tokio::time::Instant;

pub(super) struct RootLockWait {
    deadline: Instant,
    pause: Duration,
}

impl RootLockWait {
    pub(super) const LIMIT: Duration = Duration::from_secs(2);
    const FIRST_PAUSE: Duration = Duration::from_millis(10);
    const LONGEST_PAUSE: Duration = Duration::from_millis(200);

    pub(super) fn new() -> Self {
        Self {
            deadline: Instant::now() + Self::LIMIT,
            pause: Self::FIRST_PAUSE,
        }
    }

    pub(super) async fn pause(&mut self) -> bool {
        let time_left = self.deadline.saturating_duration_since(Instant::now());
        let paused = Self::sleep_within(self.pause, time_left).await;
        self.pause = (self.pause * 2).min(Self::LONGEST_PAUSE);
        paused
    }

    async fn sleep_within(pause: Duration, time_left: Duration) -> bool {
        if time_left.is_zero() {
            return false;
        }
        tokio::time::sleep(pause.min(time_left)).await;
        true
    }
}
