use super::WarmProcess;
use crate::Failure;

pub(super) struct WarmAnswer<T> {
    pub(super) result: Result<T, Failure>,
    pub(super) kept: Option<WarmProcess>,
}

impl<T> WarmAnswer<T> {
    pub(super) async fn after(
        result: Result<T, Failure>,
        process: WarmProcess,
        relay_failed: bool,
    ) -> Self {
        Self::keep_when_ok(result, process, relay_failed).await
    }

    pub(super) fn without_process(result: Result<T, Failure>) -> Self {
        Self { result, kept: None }
    }

    async fn keep_when_ok(
        result: Result<T, Failure>,
        mut process: WarmProcess,
        relay_failed: bool,
    ) -> Self {
        let replaced = [result.is_err(), relay_failed].contains(&true);
        if replaced {
            process.retire().await;
            return Self::without_process(result);
        }
        process.finish_request();
        Self {
            result,
            kept: Some(process),
        }
    }
}
