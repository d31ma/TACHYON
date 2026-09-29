mod input_step;
mod socket_channels;
mod socket_input;
mod socket_output;
mod socket_reader;
mod socket_writer;

pub(crate) use socket_channels::SocketChannels;
pub(crate) use socket_input::SocketInput;
pub(crate) use socket_output::SocketOutput;

use super::super::{HandlerCancellation, HandlerSource};
use super::{Drained, HandlerSupervisor, ProcessFailure, ProcessReaper, StartedProcess};
use crate::Failure;
use command_group::AsyncGroupChild;
use socket_reader::SocketReader;
use socket_writer::SocketWriter;
use std::time::Duration;
use tachyon_contracts::HandlerRequest;
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::task::{JoinError, JoinHandle};
use tokio::time::{Instant, sleep_until, timeout_at};

pub(super) struct SocketProcess {
    child: AsyncGroupChild,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

impl SocketProcess {
    const SOCKET_GRACE: Duration = Duration::from_secs(5);
    const SOCKET_CALL_MILLISECONDS: u64 = 30_000;

    pub(super) async fn invoke(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        request: &HandlerRequest,
        channels: SocketChannels,
        cancellation: &HandlerCancellation,
        lifetime: Duration,
    ) -> Result<(), Failure> {
        supervisor
            .options
            .isolation
            .validate_backend_language(source.language())?;
        let deadline = Instant::now() + lifetime;
        let StartedProcess {
            child,
            stdin,
            stdout,
            stderr,
            resource_access,
            _adapter,
        } = supervisor
            .start_process(source, request, deadline, cancellation, true)
            .await?;
        let process = Self {
            child,
            stdin,
            stdout,
            stderr,
        };
        let stderr_limit = supervisor.options.stderr_limit;
        let result = process
            .drive(stderr_limit, request, channels, cancellation, deadline)
            .await;
        HandlerSupervisor::close_resource_access(resource_access).await;
        result
    }

    async fn drive(
        self,
        stderr_limit: usize,
        request: &HandlerRequest,
        channels: SocketChannels,
        cancellation: &HandlerCancellation,
        deadline: Instant,
    ) -> Result<(), Failure> {
        let Self {
            mut child,
            mut stdin,
            stdout,
            stderr,
        } = self;
        let request_id = request.request_id.clone();
        let stderr_task = tokio::spawn(Drained::read_all(stderr, stderr_limit));
        let written = Self::write_first_request(&mut stdin, request).await;
        Self::stop_when_unwritten(written, &mut child, &request_id, &stderr_task).await?;
        let mut writer = SocketWriter::spawn(stdin, channels.input, request_id.clone());
        let reader = SocketReader::spawn(stdout, channels.output, request_id.clone());
        let read = Self::first_finish(reader, cancellation, deadline).await;
        let read = Self::reading_result(read, cancellation);
        let result = Self::exit_after_reading(read, &mut child, deadline).await;
        ProcessReaper::terminate(&mut child, &request_id).await;
        writer.abort();
        let _writer = (&mut writer).await;
        let stderr = Drained::settle(stderr_task, Instant::now() + Self::SOCKET_GRACE).await;
        Self::socket_outcome(result, stderr)
    }

    async fn write_first_request(
        stdin: &mut ChildStdin,
        request: &HandlerRequest,
    ) -> Result<(), Failure> {
        let mut timed = request.clone();
        timed.deadline_ms = Some(Self::SOCKET_CALL_MILLISECONDS);
        let bytes = HandlerSupervisor::encode_request(false, &timed)?;
        HandlerSupervisor::write_request(stdin, &bytes).await
    }

    async fn stop_when_unwritten(
        written: Result<(), Failure>,
        child: &mut AsyncGroupChild,
        request_id: &str,
        stderr_task: &JoinHandle<Drained>,
    ) -> Result<(), Failure> {
        if let Err(failure) = written {
            ProcessReaper::terminate(child, request_id).await;
            stderr_task.abort();
            return Err(failure);
        }
        Ok(())
    }

    async fn first_finish(
        mut reader: JoinHandle<Result<(), Failure>>,
        cancellation: &HandlerCancellation,
        deadline: Instant,
    ) -> Option<Result<Result<(), Failure>, JoinError>> {
        let read = tokio::select! {
            read = &mut reader => Some(read),
            () = cancellation.cancelled() => None,
            () = sleep_until(deadline) => None,
        };
        if read.is_none() {
            reader.abort();
            let _reader = reader.await;
        }
        read
    }

    fn reading_result(
        read: Option<Result<Result<(), Failure>, JoinError>>,
        cancellation: &HandlerCancellation,
    ) -> Result<(), Failure> {
        match read {
            Some(Ok(result)) => result,
            Some(Err(_)) => Err(SocketOutput::failure("The socket reader failed.")),
            None if cancellation.is_cancelled() => Err(ProcessFailure::cancelled()),
            None => Err(ProcessFailure::timed_out()),
        }
    }

    async fn exit_after_reading(
        read: Result<(), Failure>,
        child: &mut AsyncGroupChild,
        deadline: Instant,
    ) -> Result<Option<std::process::ExitStatus>, Failure> {
        read?;
        Self::wait_for_exit(child, deadline).await
    }

    fn socket_outcome(
        result: Result<Option<std::process::ExitStatus>, Failure>,
        stderr: Result<Drained, Failure>,
    ) -> Result<(), Failure> {
        match (result, stderr) {
            (Err(failure), _) => Err(failure),
            (Ok(Some(status)), Ok(stderr)) => Err(stderr.exit_failure(status)),
            (Ok(_), _) => Ok(()),
        }
    }

    async fn wait_for_exit(
        child: &mut AsyncGroupChild,
        deadline: Instant,
    ) -> Result<Option<std::process::ExitStatus>, Failure> {
        let limit = deadline.min(Instant::now() + Self::SOCKET_GRACE);
        match timeout_at(limit, child.inner().wait()).await {
            Ok(Ok(status)) if status.success() => Ok(None),
            Ok(Ok(status)) => Ok(Some(status)),
            Ok(Err(_)) => Err(SocketOutput::failure("Cannot see the socket handler stop.")),
            Err(_) => Err(ProcessFailure::timed_out()),
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{SocketChannels, SocketProcess};
    use crate::handler::HandlerCancellation;
    use command_group::AsyncCommandGroup as _;
    use std::process::Stdio;
    use std::time::Duration;
    use tachyon_contracts::{HandlerRequest, HttpMethod};
    use tokio::process::Command;
    use tokio::sync::mpsc;
    use tokio::time::Instant;

    #[tokio::test]
    async fn a_handler_that_stops_after_the_close_frame_ends_the_socket_cleanly() {
        let mut child = Command::new("sh")
            .args(["-c", "cat > /dev/null"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .group_spawn()
            .expect("a handler that reads until its input ends");
        let process = SocketProcess {
            stdin: child.inner().stdin.take().expect("stdin"),
            stdout: child.inner().stdout.take().expect("stdout"),
            stderr: child.inner().stderr.take().expect("stderr"),
            child,
        };
        let (input, inputs) = mpsc::channel(1);
        let (output, _outputs) = mpsc::channel(1);
        drop(input);
        let request = HandlerRequest::route("socket-1", "/chat", HttpMethod::Get);
        let deadline = Instant::now() + Duration::from_secs(30);
        let finished = process
            .drive(
                4096,
                &request,
                SocketChannels {
                    input: inputs,
                    output,
                },
                &HandlerCancellation::default(),
                deadline,
            )
            .await;
        assert!(finished.is_ok(), "{finished:?}");
    }
}
