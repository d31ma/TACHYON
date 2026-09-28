use super::{Drained, ProcessFailure, ProcessReaper, StreamReadOutcome};
use crate::Failure;
use command_group::AsyncGroupChild;
use std::process::ExitStatus;
use std::time::Duration;
use tokio::process::ChildStdin;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::error::Elapsed;
use tokio::time::{Instant, timeout_at};

pub(super) struct RunningStream {
    pub(super) child: AsyncGroupChild,
    pub(super) stdin: Option<ChildStdin>,
    pub(super) events_task: JoinHandle<Result<StreamReadOutcome, Failure>>,
    pub(super) stderr_task: JoinHandle<Drained>,
    pub(super) deadline: Instant,
}

impl RunningStream {
    pub(super) async fn stop(
        self,
        request_id: &str,
        grace: Duration,
        failure: Failure,
    ) -> Result<(), Failure> {
        let Self {
            mut child,
            mut stdin,
            mut events_task,
            stderr_task,
            deadline,
        } = self;
        ProcessReaper::cancel_and_reap(&mut child, stdin.as_mut(), request_id, grace, deadline)
            .await;
        Self::abort_and_wait(&mut events_task).await;
        let _stderr = Drained::settle(stderr_task, deadline).await;
        Err(failure)
    }

    pub(super) async fn end_after_closed_frames(self, request_id: &str) -> Result<(), Failure> {
        let Self {
            mut child,
            stdin: _stdin,
            events_task: _events_task,
            stderr_task,
            deadline,
        } = self;
        ProcessReaper::terminate(&mut child, request_id).await;
        let _stderr = Drained::settle(stderr_task, deadline).await;
        Ok(())
    }

    pub(super) async fn end_for_closed_subscriber(self, request_id: &str) -> Result<(), Failure> {
        let Self {
            mut child,
            stdin: _stdin,
            mut events_task,
            stderr_task,
            deadline,
        } = self;
        ProcessReaper::terminate(&mut child, request_id).await;
        Self::abort_and_wait(&mut events_task).await;
        let _stderr = Drained::settle(stderr_task, deadline).await;
        Ok(())
    }

    pub(super) async fn finish_after_frames(
        self,
        frames: Result<Result<StreamReadOutcome, Failure>, JoinError>,
        request_id: &str,
    ) -> Result<(), Failure> {
        let Self {
            mut child,
            stdin: _stdin,
            events_task: _events_task,
            stderr_task,
            deadline,
        } = self;
        let status = Self::exit_after_frames(&mut child, deadline, frames).await;
        ProcessReaper::terminate(&mut child, request_id).await;
        let stderr = Drained::settle(stderr_task, deadline).await;
        Self::checked_frames(status, stderr)
    }

    pub(super) async fn finish_after_exit(
        self,
        status: std::io::Result<ExitStatus>,
        request_id: &str,
    ) -> Result<(), Failure> {
        let Self {
            mut child,
            stdin,
            mut events_task,
            stderr_task,
            deadline,
        } = self;
        drop(stdin);
        ProcessReaper::terminate(&mut child, request_id).await;
        let finished = Self::frames_after_exit(status, &mut events_task, deadline).await;
        let stderr = Drained::settle(stderr_task, deadline).await;
        Self::checked_exit(finished, stderr)
    }

    async fn abort_and_wait<T>(task: &mut JoinHandle<T>) {
        task.abort();
        let _finished = task.await;
    }

    async fn exit_after_frames(
        child: &mut AsyncGroupChild,
        deadline: Instant,
        frames: Result<Result<StreamReadOutcome, Failure>, JoinError>,
    ) -> Result<ExitStatus, Failure> {
        let frames = Self::read_frames(frames)?;
        debug_assert_eq!(frames, StreamReadOutcome::Complete);
        let waited = timeout_at(deadline, child.inner().wait()).await;
        Self::waited_status(&waited)
    }

    fn read_frames(
        frames: Result<Result<StreamReadOutcome, Failure>, JoinError>,
    ) -> Result<StreamReadOutcome, Failure> {
        match frames {
            Ok(Ok(frames)) => Ok(frames),
            Ok(Err(failure)) => Err(failure),
            Err(_) => Err(ProcessFailure::stream("Handler event reader failed.")),
        }
    }

    fn waited_status(
        waited: &Result<std::io::Result<ExitStatus>, Elapsed>,
    ) -> Result<ExitStatus, Failure> {
        match waited {
            Ok(Ok(status)) => Ok(*status),
            Ok(Err(_)) => Err(ProcessFailure::stream(
                "Cannot observe handler process exit.",
            )),
            Err(_) => Err(ProcessFailure::timed_out()),
        }
    }

    fn checked_frames(
        status: Result<ExitStatus, Failure>,
        stderr: Result<Drained, Failure>,
    ) -> Result<(), Failure> {
        match (status, stderr) {
            (Err(failure), _) | (Ok(_), Err(failure)) => Err(failure),
            (Ok(status), Ok(stderr)) => Drained::validate_stream_output(status, &stderr),
        }
    }

    async fn frames_after_exit(
        status: std::io::Result<ExitStatus>,
        events_task: &mut JoinHandle<Result<StreamReadOutcome, Failure>>,
        deadline: Instant,
    ) -> Result<Option<ExitStatus>, Failure> {
        let status = Self::observed_exit(status, events_task).await?;
        let frames = timeout_at(deadline, &mut *events_task).await;
        let frames = Self::frames_in_time(frames, events_task).await?;
        Ok(Self::status_to_check(frames, status))
    }

    async fn observed_exit(
        status: std::io::Result<ExitStatus>,
        events_task: &mut JoinHandle<Result<StreamReadOutcome, Failure>>,
    ) -> Result<ExitStatus, Failure> {
        if let Ok(status) = status {
            return Ok(status);
        }
        Self::abort_and_wait(events_task).await;
        Err(ProcessFailure::stream(
            "Cannot observe handler process exit.",
        ))
    }

    async fn frames_in_time(
        frames: Result<Result<Result<StreamReadOutcome, Failure>, JoinError>, Elapsed>,
        events_task: &mut JoinHandle<Result<StreamReadOutcome, Failure>>,
    ) -> Result<StreamReadOutcome, Failure> {
        match frames {
            Ok(Ok(Ok(frames))) => Ok(frames),
            Ok(Ok(Err(failure))) => Err(failure),
            Ok(Err(_)) => Err(ProcessFailure::stream("Handler event reader failed.")),
            Err(_) => {
                Self::abort_and_wait(events_task).await;
                Err(ProcessFailure::timed_out())
            }
        }
    }

    fn status_to_check(frames: StreamReadOutcome, status: ExitStatus) -> Option<ExitStatus> {
        match frames {
            StreamReadOutcome::SubscriberClosed => None,
            StreamReadOutcome::Complete => Some(status),
        }
    }

    fn checked_exit(
        finished: Result<Option<ExitStatus>, Failure>,
        stderr: Result<Drained, Failure>,
    ) -> Result<(), Failure> {
        match (finished, stderr) {
            (Err(failure), _) | (Ok(Some(_)), Err(failure)) => Err(failure),
            (Ok(None), _) => Ok(()),
            (Ok(Some(status)), Ok(stderr)) => Drained::validate_stream_output(status, &stderr),
        }
    }
}
