use super::{WarmAnswer, WarmContext, WarmProcess, WarmStop};

pub(super) struct WarmStreamEnd;

impl WarmStreamEnd {
    pub(super) async fn complete(process: WarmProcess, context: &WarmContext) -> WarmAnswer<()> {
        let stderr = process.take_stderr();
        let checked = stderr.check_diagnostics();
        let relay_failed = stderr.reports_relay_failure(&context.request_id);
        WarmAnswer::after(checked, process, relay_failed).await
    }

    pub(super) async fn abandoned(process: WarmProcess) -> WarmAnswer<()> {
        process.retire().await;
        WarmAnswer::without_process(Ok(()))
    }

    pub(super) async fn stopped(
        stop: WarmStop,
        process: WarmProcess,
        context: &WarmContext,
    ) -> WarmAnswer<()> {
        let result = stop.into_stream_result(process, context).await;
        WarmAnswer::without_process(result)
    }
}
