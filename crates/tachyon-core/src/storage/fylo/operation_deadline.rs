use std::future::Future;
use std::time::{Duration, Instant};

use tokio::task::LocalKey;

pub(super) struct OperationDeadline;

impl OperationDeadline {
    pub(super) async fn scope<Work: Future>(deadline: Instant, work: Work) -> Work::Output {
        Self::key().scope(deadline, work).await
    }

    pub(super) fn time_left() -> Duration {
        let deadline = Self::key().try_with(|deadline| *deadline).ok();
        Self::until(deadline)
    }

    fn until(deadline: Option<Instant>) -> Duration {
        let Some(deadline) = deadline else {
            return Duration::ZERO;
        };
        deadline.saturating_duration_since(Instant::now())
    }

    fn key() -> &'static LocalKey<Instant> {
        tokio::task_local! {
            static DEADLINE: Instant;
        }
        &DEADLINE
    }
}
