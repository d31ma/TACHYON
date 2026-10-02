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
        let read = Self::read_until(&mut reader, cancellation, deadline).await;
        Self::settle_reader(reader, read.is_none()).await;
        read
    }

    async fn read_until(
        reader: &mut JoinHandle<Result<(), Failure>>,
        cancellation: &HandlerCancellation,
        deadline: Instant,
    ) -> Option<Result<Result<(), Failure>, JoinError>> {
        tokio::select! {
            read = reader => Some(read),
            () = cancellation.cancelled() => None,
            () = sleep_until(deadline) => None,
        }
    }

    async fn settle_reader(reader: JoinHandle<Result<(), Failure>>, interrupted: bool) {
        if interrupted {
            reader.abort();
            let _reader = reader.await;
        }
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
    use tokio::sync::{mpsc, oneshot};
    use tokio::task::JoinHandle;
    use tokio::time::Instant;

    struct ReaderDropSignal(Option<oneshot::Sender<()>>);

    impl Drop for ReaderDropSignal {
        fn drop(&mut self) {
            if let Some(sender) = self.0.take() {
                let _sent = sender.send(());
            }
        }
    }

    async fn pending_reader() -> (
        JoinHandle<Result<(), crate::Failure>>,
        oneshot::Receiver<()>,
    ) {
        let (started, ready) = oneshot::channel();
        let (dropped, finished) = oneshot::channel();
        let reader = tokio::spawn(async move {
            let _owned = ReaderDropSignal(Some(dropped));
            started
                .send(())
                .unwrap_or_else(|()| panic!("reader must hold its resource"));
            std::future::pending::<Result<(), crate::Failure>>().await
        });
        ready
            .await
            .unwrap_or_else(|error| panic!("reader must start: {error}"));
        (reader, finished)
    }

    #[tokio::test]
    async fn cancelled_socket_reader_releases_owned_resources_before_returning() {
        let (reader, mut dropped) = pending_reader().await;
        let cancellation = HandlerCancellation::default();
        cancellation.cancel();
        let read = SocketProcess::first_finish(
            reader,
            &cancellation,
            Instant::now() + Duration::from_mins(1),
        )
        .await;
        assert!(read.is_none());
        assert_eq!(dropped.try_recv(), Ok(()));
        let failure = SocketProcess::reading_result(read, &cancellation)
            .err()
            .unwrap_or_else(|| panic!("cancellation must remain a failure"));
        assert!(
            failure
                .to_string()
                .contains("Handler invocation was cancelled.")
        );
    }

    #[tokio::test]
    async fn expired_socket_reader_releases_owned_resources_before_returning() {
        let (reader, mut dropped) = pending_reader().await;
        let cancellation = HandlerCancellation::default();
        let read = SocketProcess::first_finish(reader, &cancellation, Instant::now()).await;
        assert!(read.is_none());
        assert_eq!(dropped.try_recv(), Ok(()));
        assert!(!cancellation.is_cancelled());
        let failure = SocketProcess::reading_result(read, &cancellation)
            .err()
            .unwrap_or_else(|| panic!("deadline must remain a failure"));
        assert!(
            failure
                .to_string()
                .contains("Handler invocation exceeded its deadline.")
        );
    }

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
