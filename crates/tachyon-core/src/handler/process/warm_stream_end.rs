use super::{WarmAnswer, WarmContext, WarmProcess, WarmStop};

pub(super) struct WarmStreamEnd;

impl WarmStreamEnd {
    pub(super) async fn complete(process: WarmProcess) -> WarmAnswer<()> {
        let checked = process.take_stderr().check_diagnostics();
        WarmAnswer::after(checked, process).await
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
