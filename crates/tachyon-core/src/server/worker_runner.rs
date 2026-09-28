use super::{InvocationLog, ServerRuntime};
use crate::Failure;
use crate::handler::{HandlerCancellation, HandlerSource, HandlerSupervisor};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tachyon_contracts::{HandlerRequest, HandlerResponse, HttpMethod};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

pub(in crate::server) struct WorkerRunner {
    supervisor: Arc<HandlerSupervisor>,
    source: HandlerSource,
    relative: String,
    interval: Duration,
    runtime_cancellation: CancellationToken,
}

impl WorkerRunner {
    pub(in crate::server) fn spawn(
        worker: &crate::project::ScheduledWorker,
        supervisor: &Arc<HandlerSupervisor>,
        cancellation: &CancellationToken,
        tasks: &mut JoinSet<()>,
    ) {
        let runner = Self {
            supervisor: Arc::clone(supervisor),
            source: worker.source().clone(),
            relative: worker.relative().to_owned(),
            interval: Duration::from_secs(worker.every_seconds()),
            runtime_cancellation: cancellation.clone(),
        };
        tasks.spawn(runner.run(Duration::from_secs(worker.initial_seconds())));
    }

    async fn run(self, initial: Duration) {
        let mut wait = Some(initial);
        while let Some(delay) = wait {
            wait = self.run_once(delay).await;
        }
    }

    async fn run_once(&self, delay: Duration) -> Option<Duration> {
        let waited = self.wait_unless_cancelled(delay).await;
        let keep = self.invoke_when_running(waited).await;
        self.next_wait(keep)
    }

    async fn wait_unless_cancelled(&self, delay: Duration) -> bool {
        tokio::select! {
            biased;
            () = self.runtime_cancellation.cancelled() => false,
            () = tokio::time::sleep(delay) => true,
        }
    }

    async fn invoke_when_running(&self, waited: bool) -> bool {
        if !waited || self.runtime_cancellation.is_cancelled() {
            return false;
        }
        self.invoke().await
    }

    fn next_wait(&self, keep: bool) -> Option<Duration> {
        (keep && !self.runtime_cancellation.is_cancelled()).then_some(self.interval)
    }

    async fn invoke(&self) -> bool {
        let mut request = HandlerRequest::route(
            crate::ttid::generate(),
            format!("/{}", self.relative),
            HttpMethod::Post,
        );
        request.operation = String::from("worker.run");
        let cancellation = HandlerCancellation::default();
        let invocation = self
            .supervisor
            .invoke(&self.source, &request, &cancellation);
        tokio::pin!(invocation);
        let outcome = self.outcome_unless_stopped(invocation, &cancellation).await;
        self.report(outcome.as_ref(), &request);
        outcome.is_some()
    }

    async fn outcome_unless_stopped<F>(
        &self,
        mut invocation: Pin<&mut F>,
        cancellation: &HandlerCancellation,
    ) -> Option<Result<HandlerResponse, Failure>>
    where
        F: Future<Output = Result<HandlerResponse, Failure>>,
    {
        tokio::select! {
            biased;
            () = self.runtime_cancellation.cancelled() => {
                cancellation.cancel();
                tokio::time::timeout(ServerRuntime::WORKER_CANCELLATION_SETTLE, &mut invocation).await.ok()
            }
            result = &mut invocation => Some(result),
        }
    }

    fn report(&self, outcome: Option<&Result<HandlerResponse, Failure>>, request: &HandlerRequest) {
        match outcome {
            Some(Ok(response)) if !(200..400).contains(&response.status) => {
                eprintln!(
                    "Worker '{}' reported status {}",
                    self.relative, response.status
                );
            }
            Some(Err(failure)) => InvocationLog::log_failure(
                "worker",
                &request.request_id,
                InvocationLog::runtime_family(&self.source),
                failure,
            ),
            Some(Ok(_)) | None => {}
        }
    }
}
