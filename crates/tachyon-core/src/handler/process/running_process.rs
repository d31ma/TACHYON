use super::direct_response::DirectResponse;
use super::{
    Drained, HandlerCallLabel, HandlerCancellation, ProcessFailure, ProcessOutcome, ProcessReaper,
};
use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::frame::HandlerFrame;
use crate::handler::topic_bus::TopicOutbox;
use command_group::AsyncGroupChild;
use std::process::ExitStatus;
use std::sync::Arc;
use std::time::Duration;
use tachyon_contracts::HandlerResponse;
use tokio::process::ChildStdin;
use tokio::task::JoinHandle;
use tokio::time::{Instant, sleep_until};

pub(super) struct RunningProcess {
    pub(super) child: AsyncGroupChild,
    pub(super) stdin: Option<ChildStdin>,
    pub(super) stdout_task: JoinHandle<Drained>,
    pub(super) stderr_task: JoinHandle<Drained>,
    pub(super) deadline: Instant,
    pub(super) direct_protocol: bool,
    pub(super) topics: Option<Arc<TopicOutbox>>,
    pub(super) label: HandlerCallLabel,
}

impl RunningProcess {
    pub(super) async fn run(
        mut self,
        cancellation: &HandlerCancellation,
        grace: Duration,
    ) -> Result<HandlerResponse, Failure> {
        let outcome = self.wait(cancellation).await;
        self.finish(outcome, grace).await
    }

    async fn wait(&mut self, cancellation: &HandlerCancellation) -> ProcessOutcome {
        tokio::select! {
            status = self.child.inner().wait() => ProcessOutcome::Exit(status),
            () = cancellation.cancelled() => ProcessOutcome::Cancelled,
            () = sleep_until(self.deadline) => ProcessOutcome::TimedOut,
        }
    }

    async fn finish(
        self,
        outcome: ProcessOutcome,
        grace: Duration,
    ) -> Result<HandlerResponse, Failure> {
        match outcome {
            ProcessOutcome::Exit(status) => self.finish_after_exit(status).await,
            ProcessOutcome::Cancelled => self.stop(ProcessFailure::cancelled(), grace).await,
            ProcessOutcome::TimedOut => self.stop(ProcessFailure::timed_out(), grace).await,
        }
    }

    async fn finish_after_exit(
        self,
        status: std::io::Result<ExitStatus>,
    ) -> Result<HandlerResponse, Failure> {
        let Self {
            mut child,
            stdin,
            stdout_task,
            stderr_task,
            deadline,
            direct_protocol,
            topics,
            label,
        } = self;
        drop(stdin);
        ProcessReaper::terminate(&mut child, &label.request_id).await;
        let settled = Drained::settle_pair(stdout_task, stderr_task, deadline).await;
        let (status, stdout, stderr) = Self::exit_output(status, settled)?;
        stderr.surface_events(&label);
        Drained::validate_output(status, &stdout, &stderr)?;
        Self::response(&stdout.bytes, &label.request_id, direct_protocol, topics).await
    }

    async fn response(
        bytes: &[u8],
        request_id: &str,
        direct_protocol: bool,
        topics: Option<Arc<TopicOutbox>>,
    ) -> Result<HandlerResponse, Failure> {
        if direct_protocol {
            return DirectResponse::parse(bytes, request_id, topics).await;
        }
        HandlerFrame::response(bytes, request_id)
    }

    async fn stop(self, failure: Failure, grace: Duration) -> Result<HandlerResponse, Failure> {
        let Self {
            mut child,
            mut stdin,
            stdout_task,
            stderr_task,
            deadline,
            direct_protocol: _,
            topics: _,
            label,
        } = self;
        let request_id = label.request_id.as_str();
        ProcessReaper::cancel_and_reap(&mut child, stdin.as_mut(), request_id, grace, deadline)
            .await;
        let settled = Drained::settle_pair(stdout_task, stderr_task, Instant::now() + grace).await;
        Drained::report_settled(&settled.map(|(_stdout, stderr)| stderr), &label);
        Err(failure)
    }

    fn exit_output(
        status: std::io::Result<ExitStatus>,
        settled: Result<(Drained, Drained), Failure>,
    ) -> Result<(ExitStatus, Drained, Drained), Failure> {
        match (status, settled) {
            (Err(error), _) => Err(Failure::one(diagnostic(
                2104,
                format!("Cannot observe handler process exit: {error}"),
                None,
                None,
            ))),
            (Ok(_), Err(failure)) => Err(failure),
            (Ok(status), Ok((stdout, stderr))) => Ok((status, stdout, stderr)),
        }
    }
}
