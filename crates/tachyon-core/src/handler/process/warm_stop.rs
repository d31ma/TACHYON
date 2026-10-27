use super::{ProcessFailure, WarmContext, WarmExit, WarmProcess};
use crate::Failure;

pub(super) enum WarmStop {
    Cancelled,
    TimedOut,
    Ended,
    Broken(Failure),
}

impl WarmStop {
    pub(super) async fn into_failure(self, process: WarmProcess, context: &WarmContext) -> Failure {
        self.failure_after(process, context).await
    }

    pub(super) async fn into_stream_result(
        self,
        process: WarmProcess,
        context: &WarmContext,
    ) -> Result<(), Failure> {
        self.stream_result_after(process, context).await
    }

    async fn failure_after(self, process: WarmProcess, context: &WarmContext) -> Failure {
        let request_id = context.request_id.as_str();
        match self {
            Self::Cancelled => {
                process
                    .cancel(request_id, context.grace, context.deadline)
                    .await;
                ProcessFailure::cancelled()
            }
            Self::TimedOut => {
                process
                    .cancel(request_id, context.grace, context.deadline)
                    .await;
                ProcessFailure::timed_out()
            }
            Self::Broken(failure) => {
                process.retire().await;
                failure
            }
            Self::Ended => WarmExit::failure(process, context.deadline).await,
        }
    }

    async fn stream_result_after(
        self,
        process: WarmProcess,
        context: &WarmContext,
    ) -> Result<(), Failure> {
        match self {
            Self::Ended => WarmExit::stream_result(process, context.deadline).await,
            stop => Err(stop.into_failure(process, context).await),
        }
    }
}
