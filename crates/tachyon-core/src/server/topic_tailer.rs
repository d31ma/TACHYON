use super::{TopicFailure, TopicFiles, TopicHub, TopicState};
use futures_util::future::OptionFuture;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, BufReader};
use tokio::task::JoinError;
use tokio_util::sync::CancellationToken;

type BlockingTopicOpen = Result<Result<Option<std::fs::File>, String>, JoinError>;

pub(in crate::server) struct TopicTailer {
    files: TopicFiles,
    topic: String,
    state: Arc<TopicState>,
    cancellation: CancellationToken,
}

impl TopicTailer {
    const INVALID_RECORD: &'static str = "Topic record is not valid JSON.";

    pub(in crate::server) fn new(
        files: TopicFiles,
        topic: String,
        state: Arc<TopicState>,
        cancellation: CancellationToken,
    ) -> Self {
        Self {
            files,
            topic,
            state,
            cancellation,
        }
    }

    pub(in crate::server) async fn run(self) {
        let reader = self.wait_for_file().await;
        self.read_when_opened(reader).await;
    }

    pub(in crate::server) fn frame(position: u64, record: &str) -> Result<String, &'static str> {
        let line = Self::record_line(record)?;
        Self::check_plain_line(line)?;
        let value =
            serde_json::from_str::<serde_json::Value>(line).map_err(|_| Self::INVALID_RECORD)?;
        let canonical = serde_json::to_string(&value).map_err(|_| Self::INVALID_RECORD)?;
        Ok(format!("id: {position}\ndata: {canonical}\n\n"))
    }

    fn record_line(record: &str) -> Result<&str, &'static str> {
        let line = record.strip_suffix('\n').ok_or(Self::INVALID_RECORD)?;
        Ok(line.strip_suffix('\r').unwrap_or(line))
    }

    fn check_plain_line(line: &str) -> Result<(), &'static str> {
        if line.is_empty() || line.chars().any(char::is_control) {
            return Err(Self::INVALID_RECORD);
        }
        Ok(())
    }

    async fn wait_for_file(&self) -> Option<BufReader<tokio::fs::File>> {
        let mut finished = None;
        while finished.is_none() {
            finished = self.try_open().await;
        }
        finished.flatten()
    }

    async fn try_open(&self) -> Option<Option<BufReader<tokio::fs::File>>> {
        let proceed = !self.cancellation.is_cancelled();
        let opened = OptionFuture::from(proceed.then(|| self.open_in_background()))
            .await
            .flatten();
        self.after_open(opened).await
    }

    async fn open_in_background(&self) -> Option<BlockingTopicOpen> {
        let files = self.files.clone();
        let topic = self.topic.clone();
        let mut opened = tokio::task::spawn_blocking(move || files.open_file(&topic));
        tokio::select! {
            biased;
            () = self.cancellation.cancelled() => {
                opened.abort();
                None
            }
            result = &mut opened => Some(result),
        }
    }

    async fn after_open(
        &self,
        opened: Option<BlockingTopicOpen>,
    ) -> Option<Option<BufReader<tokio::fs::File>>> {
        match opened {
            None => Some(None),
            Some(Ok(Ok(Some(file)))) => Some(Some(BufReader::new(tokio::fs::File::from_std(file)))),
            Some(Ok(Ok(None))) => self.sleep_before_retry().await,
            Some(Err(_) | Ok(Err(_))) => {
                self.state
                    .publish_error(TopicHub::FIRST_POSITION, TopicFailure::Open);
                Some(None)
            }
        }
    }

    async fn sleep_before_retry(&self) -> Option<Option<BufReader<tokio::fs::File>>> {
        let slept = Self::sleep_unless_cancelled(TopicHub::POLL_INTERVAL, &self.cancellation).await;
        (!slept).then_some(None)
    }

    async fn sleep_unless_cancelled(duration: Duration, cancellation: &CancellationToken) -> bool {
        tokio::select! {
            () = cancellation.cancelled() => false,
            () = tokio::time::sleep(duration) => true,
        }
    }

    async fn wait_for_more_records(&self) -> bool {
        Self::sleep_unless_cancelled(TopicHub::POLL_INTERVAL, &self.cancellation).await
    }

    async fn read_when_opened(&self, reader: Option<BufReader<tokio::fs::File>>) {
        if let Some(reader) = reader {
            self.read_records(reader).await;
        }
    }

    async fn read_records(&self, mut reader: BufReader<tokio::fs::File>) {
        let mut position = TopicHub::FIRST_POSITION;
        let mut bytes = 0_u64;
        let mut record = String::new();
        let mut reading = true;
        while reading {
            reading = self
                .read_next(&mut reader, &mut position, &mut bytes, &mut record)
                .await;
        }
    }

    async fn read_next(
        &self,
        reader: &mut BufReader<tokio::fs::File>,
        position: &mut u64,
        bytes: &mut u64,
        record: &mut String,
    ) -> bool {
        let proceed = !self.cancellation.is_cancelled();
        let read = OptionFuture::from(
            proceed.then(|| Self::read_line_unless_cancelled(reader, record, &self.cancellation)),
        )
        .await
        .flatten();
        self.handle_read(read, position, bytes, record).await
    }

    async fn read_line_unless_cancelled(
        reader: &mut BufReader<tokio::fs::File>,
        record: &mut String,
        cancellation: &CancellationToken,
    ) -> Option<std::io::Result<usize>> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            result = reader.read_line(record) => Some(result),
        }
    }

    async fn handle_read(
        &self,
        read: Option<std::io::Result<usize>>,
        position: &mut u64,
        bytes: &mut u64,
        record: &mut String,
    ) -> bool {
        match read {
            None => false,
            Some(Err(_)) => {
                self.state.publish_error(*position, TopicFailure::Read);
                false
            }
            Some(Ok(0)) => self.wait_for_more_records().await,
            Some(Ok(count)) => self.accept_read(count, position, bytes, record),
        }
    }

    fn accept_read(
        &self,
        count: usize,
        position: &mut u64,
        bytes: &mut u64,
        record: &mut String,
    ) -> bool {
        *bytes = bytes.saturating_add(count as u64);
        let failure = Self::limit_failure(*bytes, record);
        self.accept_within_limits(failure, position, record)
    }

    fn limit_failure(bytes: u64, record: &str) -> Option<TopicFailure> {
        if bytes > TopicHub::MAX_LOG_BYTES {
            Some(TopicFailure::LogLimit)
        } else if record.len() > TopicHub::MAX_RECORD_BYTES {
            Some(TopicFailure::RecordLimit)
        } else {
            None
        }
    }

    fn accept_within_limits(
        &self,
        failure: Option<TopicFailure>,
        position: &mut u64,
        record: &mut String,
    ) -> bool {
        match failure {
            Some(failure) => {
                self.state.publish_error(*position, failure);
                false
            }
            None => self.accept_complete_line(position, record),
        }
    }

    fn accept_complete_line(&self, position: &mut u64, record: &mut String) -> bool {
        let complete = record.ends_with('\n');
        self.publish_when_complete(complete, position, record)
    }

    fn publish_when_complete(
        &self,
        complete: bool,
        position: &mut u64,
        record: &mut String,
    ) -> bool {
        if !complete {
            return true;
        }
        self.publish_line(position, record)
    }

    fn publish_line(&self, position: &mut u64, record: &mut String) -> bool {
        let frame = Self::frame(*position, record);
        self.publish_frame(frame, position, record)
    }

    fn publish_frame(
        &self,
        frame: Result<String, &'static str>,
        position: &mut u64,
        record: &mut String,
    ) -> bool {
        match frame {
            Err(_) => {
                self.state
                    .publish_error(*position, TopicFailure::InvalidJson);
                false
            }
            Ok(frame) => {
                self.state.publish_record(*position, frame);
                *position += 1;
                record.clear();
                true
            }
        }
    }
}
