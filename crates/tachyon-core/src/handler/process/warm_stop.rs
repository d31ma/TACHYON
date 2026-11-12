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
        match self {
            Self::Cancelled => {
                Self::cancel(process, context).await;
                ProcessFailure::cancelled()
            }
            Self::TimedOut => {
                Self::cancel(process, context).await;
                ProcessFailure::timed_out()
            }
            Self::Broken(failure) => {
                process.take_stderr().report_output(&context.label);
                process.retire().await;
                failure
            }
            Self::Ended => WarmExit::failure(process, context).await,
        }
    }

    async fn cancel(process: WarmProcess, context: &WarmContext) {
        let request_id = context.label.request_id.as_str();
        let stderr = process
            .cancel(request_id, context.grace, context.deadline)
            .await;
        stderr.report_output(&context.label);
    }

    async fn stream_result_after(
        self,
        process: WarmProcess,
        context: &WarmContext,
    ) -> Result<(), Failure> {
        match self {
            Self::Ended => WarmExit::stream_result(process, context).await,
            stop => Err(stop.into_failure(process, context).await),
        }
    }
}
