use super::SiteBuilder;
use super::{DevServerOptions, HotUpdateHub, WatchWait};
use crate::hot_update::{SourceAction, SourceChanges, SourceWatcher};
use crate::tool_restart::{ToolRestart, ToolUpdate};
use crate::{BuildResult, Failure, Project, ProjectDiscovery};
use futures_util::future::OptionFuture;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

type WatchEvent = Result<notify::Event, notify::Error>;

pub(in crate::server) struct WatchTask {
    project_root: PathBuf,
    builder: SiteBuilder,
    hot_updates: HotUpdateHub,
    watcher: SourceWatcher,
    rebuild_first: bool,
    tool_restart: Option<ToolRestart>,
}

impl WatchTask {
    const DEBOUNCE: Duration = Duration::from_millis(75);
    const ROOT_CHECK: Duration = Duration::from_secs(5);
    const NOTICE_DELIVERY: Duration = Duration::from_millis(300);

    pub(in crate::server) fn start(
        project: &Project,
        options: &DevServerOptions,
        hot_updates: &HotUpdateHub,
        rebuild_first: bool,
        tool_restart: Option<ToolRestart>,
    ) -> Result<Option<Self>, Failure> {
        options
            .watch
            .then(|| Self::watching(project, options, hot_updates, rebuild_first, tool_restart))
            .transpose()
    }

    pub(in crate::server) async fn run(mut self, cancellation: CancellationToken) {
        let keep_watching = self.rebuild_first_when_asked(&cancellation).await;
        self.watch_while(keep_watching, &cancellation).await;
    }

    fn watching(
        project: &Project,
        options: &DevServerOptions,
        hot_updates: &HotUpdateHub,
        rebuild_first: bool,
        tool_restart: Option<ToolRestart>,
    ) -> Result<Self, Failure> {
        let project_root = project.root().to_path_buf();
        let watcher = SourceWatcher::start(&project_root)?;
        Ok(Self {
            project_root,
            builder: options.site_builder(),
            hot_updates: hot_updates.clone(),
            watcher,
            rebuild_first,
            tool_restart,
        })
    }

    async fn rebuild_first_when_asked(&self, cancellation: &CancellationToken) -> bool {
        if !self.rebuild_first {
            return true;
        }
        self.rebuild_everything(cancellation).await
    }

    async fn rebuild_everything(&self, cancellation: &CancellationToken) -> bool {
        let mut changes = SourceChanges::new();
        changes.force_reload();
        self.rebuild(&changes, cancellation).await
    }

    async fn watch_while(&mut self, keep_watching: bool, cancellation: &CancellationToken) {
        let mut keep_watching = keep_watching;
        while keep_watching {
            keep_watching = self.watch_once(cancellation).await;
        }
    }

    async fn watch_once(&mut self, cancellation: &CancellationToken) -> bool {
        let changes = self.receive_changes(cancellation).await;
        self.handle_changes(changes, cancellation).await
    }

    async fn handle_changes(
        &self,
        changes: Option<SourceChanges>,
        cancellation: &CancellationToken,
    ) -> bool {
        match changes {
            None => false,
            Some(changes) => self.after_changes(&changes, cancellation).await,
        }
    }

    async fn after_changes(
        &self,
        changes: &SourceChanges,
        cancellation: &CancellationToken,
    ) -> bool {
        let restarting = self
            .restart_for_tool_update(!changes.is_empty(), cancellation)
            .await;
        self.rebuild_unless_restarting(restarting, changes, cancellation)
            .await
    }

    async fn rebuild_unless_restarting(
        &self,
        restarting: bool,
        changes: &SourceChanges,
        cancellation: &CancellationToken,
    ) -> bool {
        if restarting {
            false
        } else if changes.is_empty() {
            true
        } else {
            self.rebuild(changes, cancellation).await
        }
    }

    async fn receive_changes(&mut self, cancellation: &CancellationToken) -> Option<SourceChanges> {
        let waited = Self::wait_for_event(&mut self.watcher, cancellation).await;
        self.changes_after_wait(waited, cancellation).await
    }

    async fn wait_for_event(
        watcher: &mut SourceWatcher,
        cancellation: &CancellationToken,
    ) -> WatchWait {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => WatchWait::Cancelled,
            event = watcher.receive() => event.map_or(WatchWait::Closed, WatchWait::Changed),
            () = tokio::time::sleep(Self::ROOT_CHECK) => WatchWait::Idle,
        }
    }

    async fn changes_after_wait(
        &mut self,
        waited: WatchWait,
        cancellation: &CancellationToken,
    ) -> Option<SourceChanges> {
        match waited {
            WatchWait::Cancelled => None,
            WatchWait::Idle => Self::check_project_root(&self.project_root, cancellation),
            WatchWait::Closed => {
                cancellation.cancel();
                None
            }
            WatchWait::Changed(event) => self.debounced_changes(event, cancellation).await,
        }
    }

    async fn debounced_changes(
        &mut self,
        event: WatchEvent,
        cancellation: &CancellationToken,
    ) -> Option<SourceChanges> {
        let mut changes = SourceChanges::new();
        Self::record(&mut changes, &self.project_root, event);
        let settled = Self::debounce(cancellation).await;
        settled.then(|| self.drained(changes))
    }

    fn record(changes: &mut SourceChanges, project_root: &Path, event: WatchEvent) {
        match event {
            Ok(event) => changes.record_event(project_root, event),
            Err(error) => {
                eprintln!("Source watcher reported an error: {error}");
                changes.force_reload();
            }
        }
    }

    async fn debounce(cancellation: &CancellationToken) -> bool {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => false,
            () = tokio::time::sleep(Self::DEBOUNCE) => true,
        }
    }

    fn drained(&mut self, mut changes: SourceChanges) -> SourceChanges {
        self.watcher.drain(&mut changes, &self.project_root);
        changes
    }

    fn check_project_root(
        project_root: &Path,
        cancellation: &CancellationToken,
    ) -> Option<SourceChanges> {
        project_root
            .is_dir()
            .then(SourceChanges::new)
            .or_else(|| Self::stop_for_missing_root(project_root, cancellation))
    }

    fn stop_for_missing_root(
        project_root: &Path,
        cancellation: &CancellationToken,
    ) -> Option<SourceChanges> {
        eprintln!(
            "Project root '{}' no longer exists; stopping the development server.",
            project_root.display()
        );
        cancellation.cancel();
        None
    }

    async fn restart_for_tool_update(
        &self,
        sources_changed: bool,
        cancellation: &CancellationToken,
    ) -> bool {
        OptionFuture::from(
            self.tool_restart
                .as_ref()
                .map(|tool_restart| self.check_tool(tool_restart, sources_changed, cancellation)),
        )
        .await
        .unwrap_or(false)
    }

    async fn check_tool(
        &self,
        tool_restart: &ToolRestart,
        sources_changed: bool,
        cancellation: &CancellationToken,
    ) -> bool {
        let update =
            Self::tool_update_unless_cancelled(tool_restart, sources_changed, cancellation).await;
        self.after_tool_update(update, tool_restart, cancellation)
            .await
    }

    async fn tool_update_unless_cancelled(
        tool_restart: &ToolRestart,
        sources_changed: bool,
        cancellation: &CancellationToken,
    ) -> Option<ToolUpdate> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            update = tool_restart.check(sources_changed) => Some(update),
        }
    }

    async fn after_tool_update(
        &self,
        update: Option<ToolUpdate>,
        tool_restart: &ToolRestart,
        cancellation: &CancellationToken,
    ) -> bool {
        match update {
            None => true,
            Some(update) => {
                self.apply_tool_update(update, tool_restart, cancellation)
                    .await
            }
        }
    }

    async fn apply_tool_update(
        &self,
        update: ToolUpdate,
        tool_restart: &ToolRestart,
        cancellation: &CancellationToken,
    ) -> bool {
        self.hot_updates.announce_tool_update(&update);
        Self::restart_when_asked(&update, tool_restart, cancellation).await
    }

    async fn restart_when_asked(
        update: &ToolUpdate,
        tool_restart: &ToolRestart,
        cancellation: &CancellationToken,
    ) -> bool {
        if *update != ToolUpdate::Restart {
            return false;
        }
        Self::restart_after_notice(tool_restart, cancellation).await
    }

    async fn restart_after_notice(
        tool_restart: &ToolRestart,
        cancellation: &CancellationToken,
    ) -> bool {
        Self::wait_for_notice_delivery(cancellation).await;
        tool_restart.request();
        true
    }

    async fn wait_for_notice_delivery(cancellation: &CancellationToken) {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => {}
            () = tokio::time::sleep(Self::NOTICE_DELIVERY) => {}
        }
    }

    async fn rebuild(&self, changes: &SourceChanges, cancellation: &CancellationToken) -> bool {
        let paths = changes.paths();
        let action = changes.action();
        let project = ProjectDiscovery::discover(&self.project_root);
        self.rebuild_project(project, action, paths, cancellation)
            .await
    }

    async fn rebuild_project(
        &self,
        project: Result<Project, Failure>,
        action: SourceAction,
        paths: Vec<String>,
        cancellation: &CancellationToken,
    ) -> bool {
        match project {
            Err(failure) => {
                self.hot_updates.publish_diagnostics(paths, &failure);
                eprint!("{failure}");
                true
            }
            Ok(project) => {
                self.build_project(&project, action, paths, cancellation)
                    .await
            }
        }
    }

    async fn build_project(
        &self,
        project: &Project,
        action: SourceAction,
        paths: Vec<String>,
        cancellation: &CancellationToken,
    ) -> bool {
        let rebuilt = Self::build_unless_cancelled(project, &self.builder, cancellation).await;
        self.publish_build(rebuilt, action, paths)
    }

    async fn build_unless_cancelled(
        project: &Project,
        builder: &SiteBuilder,
        cancellation: &CancellationToken,
    ) -> Option<Result<BuildResult, Failure>> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => None,
            rebuilt = builder.build(project) => Some(rebuilt),
        }
    }

    fn publish_build(
        &self,
        rebuilt: Option<Result<BuildResult, Failure>>,
        action: SourceAction,
        paths: Vec<String>,
    ) -> bool {
        match rebuilt {
            None => false,
            Some(Ok(result)) => {
                self.hot_updates.publish_rebuild(action, paths, &result);
                Self::print_rebuilt(&result);
                true
            }
            Some(Err(failure)) => {
                self.hot_updates.publish_diagnostics(paths, &failure);
                eprint!("{failure}");
                true
            }
        }
    }

    fn print_rebuilt(result: &BuildResult) {
        use std::io::Write as _;
        let mut stdout = std::io::stdout();
        let _ = writeln!(
            stdout,
            "Rebuilt {} route(s) ({})",
            result.route_count(),
            result.sha256()
        );
        let _ = writeln!(stdout, "{}", result.timings_line());
    }
}
