use super::{DiagnosticEntry, EventStream, ProducerTasks};
use crate::hot_update::SourceAction;
use crate::tool_restart::ToolUpdate;
use crate::{BuildResult, Failure};
use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, Response};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tachyon_contracts::{HotUpdate, HotUpdateKind};
use tokio::io::DuplexStream;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub(in crate::server) struct HotUpdateHub {
    sender: broadcast::Sender<String>,
    sequence: Arc<AtomicU64>,
    build_id: Arc<RwLock<Option<String>>>,
    project_root: Arc<PathBuf>,
    current_diagnostics: Arc<RwLock<Vec<DiagnosticEntry>>>,
    start_diagnostics: Arc<RwLock<Vec<DiagnosticEntry>>>,
}

impl HotUpdateHub {
    pub(in crate::server) const LIVE_ENDPOINT: &str = "/.tachyon/live";
    pub(in crate::server) const HOT_ENDPOINT: &str = "/.tachyon/hot";
    pub(in crate::server) const LIVE_SCRIPT_ENDPOINT: &str = "/.tachyon/live-reload.js";
    pub(in crate::server) const LIVE_RELOAD_SCRIPT_TAG: &str =
        r#"<script type="module" src="/.tachyon/live-reload.js"></script>"#;
    pub(in crate::server) const LIVE_RELOAD_CLIENT: &str = include_str!("live-reload.js");
    pub(in crate::server) const DIAGNOSTICS_ENDPOINT: &str = "/.tachyon/diagnostics";

    const ENDPOINTS: [&'static str; 4] = [
        Self::LIVE_ENDPOINT,
        Self::HOT_ENDPOINT,
        Self::LIVE_SCRIPT_ENDPOINT,
        Self::DIAGNOSTICS_ENDPOINT,
    ];
    const STREAM_BUFFER_BYTES: usize = 16 * 1_024;

    pub(in crate::server) fn new(build_id: Option<String>, project_root: &Path) -> Self {
        let (sender, _) = broadcast::channel(64);
        Self {
            sender,
            sequence: Arc::new(AtomicU64::new(Self::first_sequence())),
            build_id: Arc::new(RwLock::new(build_id)),
            project_root: Arc::new(project_root.to_path_buf()),
            current_diagnostics: Arc::new(RwLock::new(Vec::new())),
            start_diagnostics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    pub(in crate::server) fn is_endpoint(path: &str) -> bool {
        Self::ENDPOINTS.contains(&path)
    }

    pub(in crate::server) fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::Acquire)
    }

    pub(in crate::server) fn publish_diagnostics(&self, paths: Vec<String>, failure: &Failure) {
        self.remember_diagnostics(DiagnosticEntry::list(&self.project_root, failure));
        self.publish_report(paths, failure);
    }

    pub(in crate::server) fn publish_rebuild(
        &self,
        action: SourceAction,
        paths: Vec<String>,
        result: &BuildResult,
    ) {
        let (kind, boundaries) = Self::rebuild_kind(action);
        self.remember_diagnostics(Vec::new());
        self.publish(
            kind,
            Some(String::from(result.sha256())),
            paths,
            boundaries,
            None,
        );
    }

    pub(in crate::server) fn announce_tool_update(&self, update: &ToolUpdate) {
        update
            .notice()
            .iter()
            .for_each(|notice| self.announce(notice));
    }

    pub(in crate::server) fn publish_start_diagnostics(&self, failure: &Failure) {
        let entries = DiagnosticEntry::list(&self.project_root, failure);
        Self::store(&self.start_diagnostics, entries);
        self.publish_report(Vec::new(), failure);
    }

    pub(in crate::server) fn current_diagnostics(&self) -> Vec<DiagnosticEntry> {
        let mut entries = Self::read_entries(&self.start_diagnostics);
        entries.extend(Self::read_entries(&self.current_diagnostics));
        entries
    }

    fn publish_report(&self, paths: Vec<String>, failure: &Failure) {
        self.publish(
            HotUpdateKind::Diagnostics,
            None,
            paths,
            Vec::new(),
            Some(failure.report()),
        );
    }

    fn read_entries(entries: &RwLock<Vec<DiagnosticEntry>>) -> Vec<DiagnosticEntry> {
        entries
            .read()
            .map(|entries| entries.clone())
            .unwrap_or_default()
    }

    pub(in crate::server) fn diagnostics_response(&self) -> Response<Body> {
        let entries = serde_json::to_string(&self.current_diagnostics())
            .unwrap_or_else(|_| String::from("[]"));
        Response::builder()
            .header(CONTENT_TYPE, "application/json")
            .body(Body::from(entries))
            .unwrap_or_else(|_| Response::new(Body::empty()))
    }

    pub(in crate::server) fn stream(
        &self,
        request: &Request<Body>,
        producers: &ProducerTasks,
        cancellation: &CancellationToken,
    ) -> Response<Body> {
        let receiver = self.sender.subscribe();
        let reconnect_sequence = EventStream::last_event_sequence(request);
        let (writer, reader) = tokio::io::duplex(Self::STREAM_BUFFER_BYTES);
        let producer =
            self.clone()
                .send_updates(writer, receiver, reconnect_sequence, cancellation.clone());
        let _accepted = producers.spawn(producer);
        EventStream::response(reader, "Cannot open the hot-update stream.")
    }

    fn first_sequence() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| {
                u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
            })
    }

    fn build_id(&self) -> Option<String> {
        self.build_id.read().ok().and_then(|value| value.clone())
    }

    fn publish(
        &self,
        kind: HotUpdateKind,
        build_id: Option<String>,
        paths: Vec<String>,
        boundaries: Vec<String>,
        diagnostics: Option<tachyon_diagnostics::DiagnosticReport>,
    ) {
        self.remember_build_id(build_id.as_ref());
        let sequence = self.sequence.fetch_add(1, Ordering::AcqRel) + 1;
        let update = HotUpdate::v1(
            sequence,
            kind,
            build_id.or_else(|| self.build_id()),
            paths,
            boundaries,
            diagnostics,
        );
        self.broadcast(&update);
    }

    fn remember_diagnostics(&self, entries: Vec<DiagnosticEntry>) {
        Self::store(&self.current_diagnostics, entries);
    }

    fn store(target: &RwLock<Vec<DiagnosticEntry>>, entries: Vec<DiagnosticEntry>) {
        if let Ok(mut stored) = target.write() {
            *stored = entries;
        }
    }

    fn announce(&self, notice: &Failure) {
        self.publish_diagnostics(Vec::new(), notice);
        eprint!("{notice}");
    }

    fn remember_build_id(&self, build_id: Option<&String>) {
        if let Some(value) = build_id
            && let Ok(mut published) = self.build_id.write()
        {
            *published = Some(value.clone());
        }
    }

    fn broadcast(&self, update: &HotUpdate) {
        if let Ok(payload) = serde_json::to_string(update) {
            let _unused = self.sender.send(Self::sse_frame(update, &payload));
        }
    }

    fn rebuild_kind(action: SourceAction) -> (HotUpdateKind, Vec<String>) {
        match action {
            SourceAction::Css => (HotUpdateKind::Css, Vec::new()),
            SourceAction::Island { boundaries } => (HotUpdateKind::Island, boundaries),
            SourceAction::Reload => (HotUpdateKind::Reload, Vec::new()),
        }
    }

    fn reload_snapshot(&self) -> Option<String> {
        let sequence = self.sequence();
        if sequence == 0 {
            return None;
        }
        let update = HotUpdate::v1(
            sequence,
            HotUpdateKind::Reload,
            self.build_id(),
            Vec::new(),
            Vec::new(),
            None,
        );
        let payload = serde_json::to_string(&update).ok()?;
        Some(Self::sse_frame(&update, &payload))
    }

    fn sse_frame(update: &HotUpdate, payload: &str) -> String {
        format!("id: {}\nevent: hot\ndata: {payload}\n\n", update.sequence)
    }

    fn catch_up_frame(&self, reconnect_sequence: Option<u64>) -> Option<String> {
        reconnect_sequence
            .filter(|sequence| *sequence < self.sequence())
            .and_then(|_| self.reload_snapshot())
    }

    async fn send_updates(
        self,
        mut writer: DuplexStream,
        mut receiver: broadcast::Receiver<String>,
        reconnect_sequence: Option<u64>,
        cancellation: CancellationToken,
    ) {
        let connected = EventStream::write(&mut writer, b": connected\n\n", &cancellation).await;
        let catch_up = self.catch_up_frame(reconnect_sequence);
        let open = Self::write_catch_up(&mut writer, connected, catch_up, &cancellation).await;
        self.forward_updates(open, &mut writer, &mut receiver, &cancellation)
            .await;
    }

    async fn write_catch_up(
        writer: &mut DuplexStream,
        connected: bool,
        catch_up: Option<String>,
        cancellation: &CancellationToken,
    ) -> bool {
        match (connected, catch_up) {
            (false, _) => false,
            (true, None) => true,
            (true, Some(frame)) => EventStream::write(writer, frame.as_bytes(), cancellation).await,
        }
    }

    async fn forward_updates(
        &self,
        open: bool,
        writer: &mut DuplexStream,
        receiver: &mut broadcast::Receiver<String>,
        cancellation: &CancellationToken,
    ) {
        let mut open = open;
        while open {
            open = self.forward_next(writer, receiver, cancellation).await;
        }
    }

    async fn forward_next(
        &self,
        writer: &mut DuplexStream,
        receiver: &mut broadcast::Receiver<String>,
        cancellation: &CancellationToken,
    ) -> bool {
        let update = Self::next_update(receiver, cancellation).await;
        self.write_update(writer, update, cancellation).await
    }

    async fn next_update(
        receiver: &mut broadcast::Receiver<String>,
        cancellation: &CancellationToken,
    ) -> Option<Result<String, broadcast::error::RecvError>> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            update = receiver.recv() => Some(update),
        }
    }

    async fn write_update(
        &self,
        writer: &mut DuplexStream,
        update: Option<Result<String, broadcast::error::RecvError>>,
        cancellation: &CancellationToken,
    ) -> bool {
        match update {
            None | Some(Err(broadcast::error::RecvError::Closed)) => false,
            Some(Ok(frame)) => EventStream::write(writer, frame.as_bytes(), cancellation).await,
            Some(Err(broadcast::error::RecvError::Lagged(_))) => {
                self.write_snapshot(writer, cancellation).await
            }
        }
    }

    async fn write_snapshot(
        &self,
        writer: &mut DuplexStream,
        cancellation: &CancellationToken,
    ) -> bool {
        let frame = self.reload_snapshot();
        Self::write_when_present(writer, frame, cancellation).await
    }

    async fn write_when_present(
        writer: &mut DuplexStream,
        frame: Option<String>,
        cancellation: &CancellationToken,
    ) -> bool {
        match frame {
            None => true,
            Some(frame) => EventStream::write(writer, frame.as_bytes(), cancellation).await,
        }
    }
}
