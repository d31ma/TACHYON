use super::{
    DevServerOptions, ErrorPage, ErrorPageWait, HotUpdateHub, ProducerTasks, ServerListener,
    WatchWait,
};
use crate::hot_update::{SourceChanges, SourceWatcher};
use crate::tool_restart::ToolUpdate;
use crate::{Failure, ToolRestart};
use axum::Router;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

pub struct ErrorPageServer {
    address: SocketAddr,
    project_root: PathBuf,
    hot_updates: HotUpdateHub,
    watcher: SourceWatcher,
    tool_restart: Option<ToolRestart>,
    stopping: CancellationToken,
    producers: ProducerTasks,
    serving: JoinHandle<()>,
}

impl std::fmt::Debug for ErrorPageServer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ErrorPageServer")
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}

impl ErrorPageServer {
    const SERVER_NAME: &str = "development server";
    const IDLE_CHECK: Duration = Duration::from_secs(5);
    const DEBOUNCE: Duration = Duration::from_millis(75);
    const NOTICE_DELIVERY: Duration = Duration::from_millis(300);
    const STOP_LIMIT: Duration = Duration::from_secs(2);

    pub async fn bind(project: &Path, options: &DevServerOptions) -> Result<Self, Failure> {
        let canonical_root =
            std::fs::canonicalize(project).unwrap_or_else(|_| project.to_path_buf());
        let project_root = canonical_root.as_path();
        ServerListener::validate_exposure(
            options.host,
            options.allow_non_loopback,
            Self::SERVER_NAME,
        )?;
        let watcher = SourceWatcher::start(project_root)?;
        let (listener, address) =
            ServerListener::bind(options.host, options.port, Self::SERVER_NAME).await?;
        let hot_updates = HotUpdateHub::new(None, project_root);
        let stopping = CancellationToken::new();
        let producers = ProducerTasks::new();
        let page = ErrorPage {
            hot_updates: hot_updates.clone(),
            producers: producers.clone(),
            stopping: stopping.clone(),
        };
        let serving = tokio::spawn(Self::serve(listener, page.into_router(), stopping.clone()));
        Ok(Self {
            address,
            project_root: project_root.to_path_buf(),
            hot_updates,
            watcher,
            tool_restart: Self::tool_restart(options),
            stopping,
            producers,
            serving,
        })
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn show(&self, failure: &Failure) {
        self.hot_updates.publish_diagnostics(Vec::new(), failure);
    }

    pub async fn wait_for_change(&mut self) -> ErrorPageWait {
        let waited = self.first_change().await;
        Self::deliver_notice(&waited).await;
        waited
    }

    pub async fn stop(self) {
        self.stopping.cancel();
        self.producers.close_and_abort();
        let mut serving = self.serving;
        let _finished = tokio::time::timeout(Self::STOP_LIMIT, &mut serving).await;
        serving.abort();
    }

    async fn first_change(&mut self) -> ErrorPageWait {
        let mut waited = None;
        while waited.is_none() {
            waited = self.next_change().await;
        }
        waited.unwrap_or(ErrorPageWait::SourcesChanged)
    }

    async fn next_change(&mut self) -> Option<ErrorPageWait> {
        let sources_changed = self.sources_changed().await;
        let tool_restart = self.updated_tool(sources_changed).await;
        tool_restart
            .map(ErrorPageWait::ToolUpdated)
            .or_else(|| sources_changed.then_some(ErrorPageWait::SourcesChanged))
    }

    async fn deliver_notice(waited: &ErrorPageWait) {
        tokio::time::sleep(Self::delivery_time(waited)).await;
    }

    const fn delivery_time(waited: &ErrorPageWait) -> Duration {
        match waited {
            ErrorPageWait::ToolUpdated(_) => Self::NOTICE_DELIVERY,
            ErrorPageWait::SourcesChanged => Duration::ZERO,
        }
    }

    fn tool_restart(options: &DevServerOptions) -> Option<ToolRestart> {
        options.restart_executable.clone().map(|executable| {
            ToolRestart::watch(executable, options.restart_check_arguments.clone())
        })
    }

    async fn serve(listener: TcpListener, router: Router, stopping: CancellationToken) {
        let served = axum::serve(listener, router)
            .with_graceful_shutdown(stopping.cancelled_owned())
            .await;
        if let Err(error) = served {
            eprintln!("The error page stopped unexpectedly: {error}");
        }
    }

    async fn sources_changed(&mut self) -> bool {
        let waited = self.next_event().await;
        self.changed_after(waited).await
    }

    async fn next_event(&mut self) -> WatchWait {
        tokio::select! {
            event = self.watcher.receive() => event.map_or(WatchWait::Closed, WatchWait::Changed),
            () = tokio::time::sleep(Self::IDLE_CHECK) => WatchWait::Idle,
        }
    }

    async fn changed_after(&mut self, waited: WatchWait) -> bool {
        match waited {
            WatchWait::Changed(event) => self.debounced(event).await,
            WatchWait::Closed => Self::after_idle_check().await,
            WatchWait::Idle | WatchWait::Cancelled => false,
        }
    }

    async fn debounced(&mut self, event: Result<notify::Event, notify::Error>) -> bool {
        let mut changes = event.map_or_else(
            |_| Self::reload_changes(),
            |event| Self::event_changes(&self.project_root, event),
        );
        tokio::time::sleep(Self::DEBOUNCE).await;
        self.watcher.drain(&mut changes, &self.project_root);
        !changes.is_empty()
    }

    fn reload_changes() -> SourceChanges {
        let mut changes = SourceChanges::new();
        changes.force_reload();
        changes
    }

    fn event_changes(project_root: &Path, event: notify::Event) -> SourceChanges {
        let mut changes = SourceChanges::new();
        changes.record_event(project_root, event);
        changes
    }

    async fn after_idle_check() -> bool {
        tokio::time::sleep(Self::IDLE_CHECK).await;
        true
    }

    async fn updated_tool(&self, sources_changed: bool) -> Option<ToolRestart> {
        let tool_restart = self.tool_restart.as_ref()?;
        let update = tool_restart.check(sources_changed).await;
        self.hot_updates.announce_tool_update(&update);
        (update == ToolUpdate::Restart).then(|| tool_restart.clone())
    }
}
