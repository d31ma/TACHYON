mod access_verdict;
mod controller_dispatch;
mod dev_server_options;
mod diagnostic_entry;
mod dispatch;
mod dispatch_route;
mod error_page;
mod error_page_server;
mod error_page_wait;
mod event_stream;
mod forwarded_headers;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod forwarded_headers_tests;
mod handler_event_bridge;
mod header_names;
mod host_guard;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod host_guard_tests;
mod hot_update_hub;
mod identity_answer;
mod invocation_log;
mod middleware;
mod middleware_outcome;
mod preview_dispatch;
mod preview_server;
mod preview_server_options;
mod protocol_request;
mod route_guard;
mod route_invocation;
mod server_listener;
mod server_response;
mod server_runtime;
mod socket_dispatch;
mod source_location;
mod static_files;
#[cfg(test)]
#[allow(clippy::expect_used)]
mod static_files_tests;
mod topic_admission;
mod topic_failure;
mod topic_files;
mod topic_hub;
mod topic_replay;
mod topic_state;
mod topic_stream;
mod topic_tailer;
mod watch_task;
mod watch_wait;
mod worker_runner;

pub use dev_server_options::DevServerOptions;
pub use error_page_server::ErrorPageServer;
pub use error_page_wait::ErrorPageWait;
pub use preview_server::PreviewServer;
pub use preview_server_options::PreviewServerOptions;
pub use route_invocation::RouteInvocation;

use crate::failure::diagnostic;
use crate::handler::{HandlerSupervisor, RuntimeRequirements};
use crate::tool_restart::ToolRestart;
use crate::{BuildResult, Failure, Project, ProjectDiscovery, WebCompiler};
use access_verdict::AccessVerdict;
use axum::Router;
use diagnostic_entry::DiagnosticEntry;
use dispatch::Dispatch;
use dispatch_route::DispatchRoute;
use error_page::ErrorPage;
use event_stream::EventStream;
use forwarded_headers::ForwardedHeaders;
use futures_util::future::OptionFuture;
use handler_event_bridge::HandlerEventBridge;
use header_names::HeaderNames;
use host_guard::HostGuard;
use hot_update_hub::HotUpdateHub;
use identity_answer::IdentityAnswer;
use invocation_log::InvocationLog;
use middleware::Middleware;
use middleware_outcome::MiddlewareOutcome;
use preview_dispatch::PreviewDispatch;
use protocol_request::ProtocolRequest;
use route_guard::RouteGuard;
use server_listener::ServerListener;
use server_response::ServerResponse;
use server_runtime::{BackgroundPlan, ProducerTasks, ServerRuntime};
use source_location::SourceLocation;
use static_files::StaticFiles;
use std::future::{Future, IntoFuture};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use topic_admission::TopicAdmission;
use topic_failure::TopicFailure;
use topic_files::TopicFiles;
use topic_hub::TopicHub;
use topic_replay::TopicReplay;
use topic_state::{TopicRecord, TopicState};
use topic_stream::TopicStream;
use topic_tailer::TopicTailer;
use watch_task::WatchTask;
use watch_wait::WatchWait;
use worker_runner::WorkerRunner;

pub struct DevServer {
    listener: Option<TcpListener>,
    application: Option<Router>,
    address: SocketAddr,
    public_host: crate::ServerHost,
    build: Option<BuildResult>,
    runtime: ServerRuntime,
    tool_restart: Option<ToolRestart>,
    hot_updates: HotUpdateHub,
}

impl std::fmt::Debug for DevServer {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DevServer")
            .field("address", &self.address)
            .field("build", &self.build)
            .field("background_started", &self.runtime.started())
            .finish_non_exhaustive()
    }
}

impl DevServer {
    pub async fn bind(
        project_root: impl AsRef<Path>,
        options: &DevServerOptions,
    ) -> Result<Self, Failure> {
        Self::validate_dev_exposure(options)?;
        let project = ProjectDiscovery::discover(project_root)?;
        Self::bind_project(&project, options).await
    }

    pub async fn bind_project(
        project: &Project,
        options: &DevServerOptions,
    ) -> Result<Self, Failure> {
        let application = crate::ApplicationConfig::load_project_for_server(
            project,
            Some(options.host.into()),
            Some(options.port),
        )?;
        Self::bind_project_with_application(project, options, application).await
    }

    pub async fn bind_with_application(
        project_root: impl AsRef<Path>,
        options: &DevServerOptions,
        application: crate::ApplicationConfig,
    ) -> Result<Self, Failure> {
        Self::validate_dev_exposure(options)?;
        let project = ProjectDiscovery::discover(project_root)?;
        Self::bind_project_with_application(&project, options, application).await
    }

    pub async fn bind_project_with_application(
        project: &Project,
        options: &DevServerOptions,
        application: crate::ApplicationConfig,
    ) -> Result<Self, Failure> {
        Self::validate_dev_exposure(options)?;
        let mut policy = crate::HandlerSupervisorOptions::from_environment()?;
        policy.application = application;
        let supervisor = Arc::new(HandlerSupervisor::new(policy)?);
        let hosts = crate::safety::DeclaredHosts::yon(&project.capability());
        supervisor.send_outgoing_requests_through(crate::handler::EgressProxy::start(hosts).await?);
        Self::bind_project_with_supervisor(project, options, supervisor).await
    }

    #[must_use]
    pub const fn address(&self) -> SocketAddr {
        self.address
    }

    #[must_use]
    pub fn url(&self) -> String {
        self.public_host.url(self.address)
    }

    #[must_use]
    pub const fn build(&self) -> Option<&BuildResult> {
        self.build.as_ref()
    }

    #[must_use]
    pub fn tool_restart(&self) -> Option<ToolRestart> {
        self.tool_restart.clone()
    }

    pub fn show_start_diagnostics(&self, failure: &Failure) {
        self.hot_updates.publish_start_diagnostics(failure);
    }

    pub async fn run_until<F>(mut self, shutdown: F) -> Result<(), Failure>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.runtime.start();
        let parts = self.listener.take().zip(self.application.take());
        self.serve_parts(parts, shutdown).await
    }

    async fn bind_project_with_supervisor(
        project: &Project,
        options: &DevServerOptions,
        supervisor: Arc<HandlerSupervisor>,
    ) -> Result<Self, Failure> {
        let requirements = RuntimeRequirements::from_sources(project.invocation_sources());
        supervisor.preflight(&requirements).await?;
        let (routes, page_routes) = DispatchRoute::discover(project);
        let (build, output_directory) = Self::prepare_dev_output(project, options).await?;
        let (listener, address) =
            ServerListener::bind(options.host, options.port, "development server").await?;
        let middleware = project.middleware().cloned().map(Arc::new);
        let identity = project.identity_provider().cloned().map(Arc::new);
        let build_id = build.as_ref().map(|result| String::from(result.sha256()));
        let hot_updates = HotUpdateHub::new(build_id, project.root());
        let rebuild_first = Self::rebuilds_first(options, build.as_ref());
        let tool_restart = options
            .restart_executable
            .clone()
            .filter(|_| options.watch)
            .map(|executable| {
                ToolRestart::watch(executable, options.restart_check_arguments.clone())
            });
        let watcher = WatchTask::start(
            project,
            options,
            &hot_updates,
            rebuild_first,
            tool_restart.clone(),
        )?;
        let topic_folder =
            crate::handler::TopicFolder::prepare(&project.capability(), supervisor.application())?;
        let runtime = ServerRuntime::new(BackgroundPlan {
            supervisor: Arc::clone(&supervisor),
            workers: project.workers().to_vec(),
            watcher,
            topics: crate::handler::TopicDeliveries::with_folder(
                topic_folder.clone(),
                &supervisor,
                project.invocation_sources(),
            ),
        });
        let topic_hub = TopicHub::open(topic_folder, &runtime)?;
        let dispatch_state = Dispatch {
            routes: Arc::new(routes),
            page_routes: Arc::new(page_routes),
            supervisor: Arc::clone(&supervisor),
            files: StaticFiles::directory(&output_directory, !options.watch),
            hot_updates: hot_updates.clone(),
            watch: options.watch,
            middleware,
            identity,
            topic_hub,
            sockets: socket_dispatch::SocketHub::default(),
            cancellation: runtime.cancellation.clone(),
            producers: runtime.producers.clone(),
        };
        let page_hosts = crate::safety::DeclaredHosts::tac(&project.capability());
        let guard = HostGuard::for_project(
            options.host,
            &project.capability(),
            supervisor.application().hosts(),
        );
        let edge = ForwardedHeaders::new(supervisor.application().trusted_proxies().clone());
        let application = dispatch_state.into_router(&page_hosts, guard, edge);
        let public_host = Self::public_host(options, supervisor.application());
        Ok(Self {
            listener: Some(listener),
            application: Some(application),
            address,
            public_host,
            build,
            runtime,
            tool_restart,
            hot_updates,
        })
    }

    fn public_host(
        options: &DevServerOptions,
        application: &crate::ApplicationConfig,
    ) -> crate::ServerHost {
        application
            .server_host()
            .filter(|host| host.address() == options.host)
            .cloned()
            .unwrap_or_else(|| options.host.into())
    }

    fn rebuilds_first(options: &DevServerOptions, build: Option<&BuildResult>) -> bool {
        options.build && build.is_none()
    }

    fn validate_dev_exposure(options: &DevServerOptions) -> Result<(), Failure> {
        ServerListener::validate_exposure(
            options.host,
            options.allow_non_loopback,
            "development server",
        )
    }

    async fn prepare_dev_output(
        project: &Project,
        options: &DevServerOptions,
    ) -> Result<(Option<BuildResult>, PathBuf), Failure> {
        let last_build = project.root().join(&options.output_directory);
        let serve_last_build = Self::serves_last_build(options, &last_build);
        let builds = Self::builds_before_serving(options, serve_last_build);
        let build_options = options.build_options();
        let build = OptionFuture::from(
            builds.then(|| WebCompiler::build_project_async(project, &build_options)),
        )
        .await
        .transpose()?;
        let output_directory = build.as_ref().map_or_else(
            || project.root().join(&options.output_directory),
            |result| result.output_directory().to_path_buf(),
        );
        Self::check_output_directory(&output_directory)?;
        Ok((build, output_directory))
    }

    fn serves_last_build(options: &DevServerOptions, last_build: &Path) -> bool {
        options.watch && WebCompiler::has_previous_build(last_build)
    }

    fn builds_before_serving(options: &DevServerOptions, serve_last_build: bool) -> bool {
        options.build && !serve_last_build
    }

    fn check_output_directory(output_directory: &Path) -> Result<(), Failure> {
        if std::fs::metadata(output_directory).is_ok_and(|metadata| metadata.is_dir()) {
            return Ok(());
        }
        Err(Failure::one(diagnostic(
            1304,
            format!(
                "Cannot serve missing bundle directory {}.",
                output_directory.display()
            ),
            Some(String::from("Run `ty bundle` or remove --no-bundle.")),
            None,
        )))
    }

    async fn serve_parts<F>(
        mut self,
        parts: Option<(TcpListener, Router)>,
        shutdown: F,
    ) -> Result<(), Failure>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        if let Some((listener, application)) = parts {
            return self.serve_and_stop(listener, application, shutdown).await;
        }
        self.runtime
            .shutdown_until(Instant::now() + ServerRuntime::SHUTDOWN)
            .await;
        Err(Failure::one(diagnostic(
            1303,
            "Development server ownership was already consumed.",
            Some(String::from(
                "Bind a new development server before serving.",
            )),
            None,
        )))
    }

    async fn serve_and_stop<F>(
        &mut self,
        listener: TcpListener,
        application: Router,
        shutdown: F,
    ) -> Result<(), Failure>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let (served, deadline) = Self::serve_until_shutdown(
            listener,
            application,
            &self.runtime.producers,
            self.runtime.cancellation.clone(),
            shutdown,
        )
        .await;
        self.runtime.shutdown_until(deadline).await;
        Self::served_result(served)
    }

    fn served_result(served: Option<std::io::Result<()>>) -> Result<(), Failure> {
        match served {
            Some(Ok(())) | None => Ok(()),
            Some(Err(error)) => Err(Failure::one(diagnostic(
                1303,
                format!("Development server stopped unexpectedly: {error}"),
                Some(String::from(
                    "Restart the server and inspect the reported I/O error.",
                )),
                None,
            ))),
        }
    }

    async fn serve_until_shutdown<F>(
        listener: TcpListener,
        application: Router,
        producers: &ProducerTasks,
        cancellation: CancellationToken,
        shutdown: F,
    ) -> (Option<std::io::Result<()>>, Instant)
    where
        F: Future<Output = ()>,
    {
        let graceful_cancellation = cancellation.clone();
        let served = axum::serve(
            listener,
            application.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            graceful_cancellation.cancelled().await;
        })
        .into_future();
        tokio::pin!(served);
        tokio::pin!(shutdown);
        tokio::select! {
            result = &mut served => {
                (Some(result), Instant::now() + ServerRuntime::SHUTDOWN)
            }
            () = &mut shutdown => {
                producers.close();
                cancellation.cancel();
                let deadline = Instant::now() + ServerRuntime::SHUTDOWN;
                let graceful_deadline = ServerRuntime::cooperative_shutdown_deadline(deadline);
                (tokio::time::timeout_at(graceful_deadline, &mut served).await.ok(), deadline)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::{
        DevServer, DevServerOptions, HotUpdateHub, InvocationLog, PreviewServer,
        PreviewServerOptions, ProducerTasks, ServerResponse, ServerRuntime, StaticFiles,
        TopicFailure, TopicHub, TopicState, TopicStream, TopicTailer,
    };
    use crate::compiler::hex_digest;
    use crate::failure::diagnostic;
    use crate::handler::{HandlerRuntimePrograms, HandlerSupervisor, HandlerSupervisorOptions};
    use crate::{Failure, Project, ProjectDiscovery};
    use axum::body::{Body, Bytes, to_bytes};
    use axum::http::{Request, StatusCode};
    use sha2::{Digest, Sha256};
    use std::fs;
    use std::io::Write as _;
    use std::net::{IpAddr, Ipv4Addr, TcpListener as StandardTcpListener};
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::{Duration, Instant};
    use tempfile::TempDir;
    use tokio_util::sync::CancellationToken;
    use tower_http::services::ServeDir;

    fn topic_project(root: &Path) -> Arc<cap_std::fs::Dir> {
        Arc::new(
            cap_std::fs::Dir::open_ambient_dir(root, cap_std::ambient_authority())
                .expect("topic project capability"),
        )
    }

    fn topic_folder(root: &Path) -> crate::handler::TopicFolder {
        crate::handler::TopicFolder::resolve(topic_project(root), None).expect("topic folder")
    }

    fn topic_hub(root: &Path) -> (TopicHub, CancellationToken, ProducerTasks) {
        let cancellation = CancellationToken::new();
        let producers = ProducerTasks::new();
        let hub = TopicHub::new(topic_folder(root), cancellation.clone(), producers.clone())
            .expect("topic hub");
        (hub, cancellation, producers)
    }

    fn topic_directory(root: &Path) -> PathBuf {
        let directory = root.join(TopicHub::DIRECTORY);
        fs::create_dir_all(&directory).expect("topic directory");
        directory
    }

    fn replay_length(topic: &TopicState) -> usize {
        topic.replay.lock().expect("replay").len()
    }

    async fn wait_until(description: &str, mut condition: impl FnMut() -> bool) {
        tokio::time::timeout(Duration::from_secs(1), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect(description);
    }

    fn append_record(path: &Path, record: &[u8]) {
        fs::OpenOptions::new()
            .append(true)
            .open(path)
            .expect("append log")
            .write_all(record)
            .expect("append record");
    }

    fn topic_request(uri: &str, last_event_id: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder().uri(uri);
        if let Some(last_event_id) = last_event_id {
            builder = builder.header("last-event-id", last_event_id);
        }
        builder.body(Body::empty()).expect("request")
    }

    fn runtime_without_plan() -> ServerRuntime {
        ServerRuntime {
            supervisor: Arc::new(
                HandlerSupervisor::new(HandlerSupervisorOptions::default()).expect("supervisor"),
            ),
            cancellation: CancellationToken::new(),
            tasks: tokio::task::JoinSet::new(),
            producers: ProducerTasks::new(),
            plan: None,
        }
    }

    fn write_page_project(root: &Path) {
        for (relative, contents) in [
            ("client/pages/tac.html", "<page-view />"),
            ("client/components/page/view/tac.html", "<main>Page</main>"),
        ] {
            let source = root.join(relative);
            fs::create_dir_all(source.parent().expect("parent")).expect("directory");
            fs::write(source, contents).expect("source");
        }
    }

    fn unbuilt_options() -> DevServerOptions {
        DevServerOptions {
            port: 0,
            build: false,
            watch: false,
            ..DevServerOptions::default()
        }
    }

    #[tokio::test]
    async fn hot_update_script_serves_the_event_connection_and_document_lifecycle() {
        let response = ServerResponse::live_script();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers()["content-type"],
            "text/javascript; charset=utf-8"
        );
        let body = to_bytes(response.into_body(), 16 * 1024)
            .await
            .expect("hot update script");
        let source = std::str::from_utf8(&body).expect("JavaScript source");
        assert_eq!(source, HotUpdateHub::LIVE_RELOAD_CLIENT);
        assert_eq!(body.len(), 5481);
        assert_eq!(
            hex_digest(Sha256::digest(&body)),
            "0965a5f3c6698b86d59a2c93e50000d54d13f3f50b7fa3770d5c159763c765f9"
        );
    }

    #[tokio::test]
    async fn topic_hub_reuses_one_tailer_and_preserves_replay_order() {
        let root = tempfile::tempdir().expect("topic root");
        let directory = topic_directory(root.path());
        let log = directory.join("orders.jsonl");
        fs::write(&log, "{\"value\":1}\n{\"value\":2}\n").expect("initial log");
        let (hub, cancellation, producers) = topic_hub(root.path());
        let (topic, first) = hub.subscribe("orders").expect("first subscriber");
        let (_same, second) = hub.subscribe("orders").expect("second subscriber");
        wait_until("tailer read", || replay_length(&topic) == 2).await;
        {
            let replay = topic.replay.lock().expect("replay");
            assert_eq!(replay[0].0, 0);
            assert!(replay[0].1.contains("data: {\"value\":1}"));
            assert_eq!(replay[1].0, 1);
            assert!(replay[1].1.contains("data: {\"value\":2}"));
        }
        assert_eq!(producers.len(), 1, "one physical tailer per topic");
        let retained = directory.join("orders-retained.jsonl");
        fs::rename(&log, &retained).expect("retain opened inode");
        fs::write(&log, "{\"planted\":true}\n").expect("replacement log");
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(replay_length(&topic), 2, "path replacement was followed");
        append_record(&retained, b"{\"value\":3}\n");
        wait_until("hot append", || replay_length(&topic) == 3).await;
        drop((first, second));
        cancellation.cancel();
        tokio::time::sleep(Duration::from_millis(100)).await;
        producers.reap_finished();
        assert_eq!(producers.len(), 0, "tailer settled after shutdown");
    }

    #[tokio::test]
    async fn topic_hub_enforces_per_topic_admission() {
        let root = tempfile::tempdir().expect("topic root");
        let (hub, cancellation, _producers) = topic_hub(root.path());
        let mut admissions = Vec::new();
        for _ in 0..TopicHub::MAX_SUBSCRIBERS_PER_TOPIC {
            admissions.push(hub.subscribe("bounded").expect("within limit").1);
        }
        assert!(hub.subscribe("bounded").is_err());
        drop(admissions);
        cancellation.cancel();
    }

    #[tokio::test]
    async fn topic_hub_retires_more_than_the_active_topic_limit_sequentially() {
        let root = tempfile::tempdir().expect("topic root");
        let (hub, cancellation, producers) = topic_hub(root.path());
        for index in 0..(TopicHub::MAX_ACTIVE_TOPICS * 3) {
            let name = format!("topic-{index}");
            let (_, admission) = hub.subscribe(&name).expect("sequential admission");
            drop(admission);
            assert!(hub.topics.lock().expect("topics").is_empty());
        }
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                producers.reap_finished();
                if producers.len() == 0 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("retired tailers settled");
        assert_eq!(hub.subscribers.load(Ordering::Acquire), 0);
        cancellation.cancel();
    }

    #[tokio::test]
    async fn topic_hub_concurrent_resubscribe_keeps_the_current_generation() {
        let root = tempfile::tempdir().expect("topic root");
        let (hub, cancellation, _producers) = topic_hub(root.path());
        let (first_state, first) = hub.subscribe("race").expect("first subscriber");
        let (ready_sender, ready_receiver) = tokio::sync::oneshot::channel();
        let (release_sender, release_receiver) = tokio::sync::oneshot::channel();
        let other_hub = hub.clone();
        let subscriber = tokio::spawn(async move {
            let (state, admission) = other_hub.subscribe("race").expect("resubscriber");
            assert!(ready_sender.send(state).is_ok(), "state receiver");
            let _ = release_receiver.await;
            drop(admission);
        });
        let second_state = ready_receiver.await.expect("resubscriber ready");
        assert!(Arc::ptr_eq(&first_state, &second_state));
        drop(first);
        let current = hub.topics.lock().expect("topics").get("race").cloned();
        assert!(current.is_some_and(|state| Arc::ptr_eq(&state, &second_state)));
        release_sender.send(()).expect("release subscriber");
        subscriber.await.expect("resubscriber task");
        assert!(hub.topics.lock().expect("topics").is_empty());

        let (replacement, replacement_admission) =
            hub.subscribe("race").expect("replacement generation");
        assert!(!Arc::ptr_eq(&first_state, &replacement));
        assert_eq!(replay_length(&replacement), 0);
        drop(replacement_admission);
        cancellation.cancel();
    }

    #[tokio::test]
    async fn topic_hub_marks_a_slow_subscriber_lagged_without_growing_its_queue() {
        let root = tempfile::tempdir().expect("topic root");
        let (hub, cancellation, _producers) = topic_hub(root.path());
        let (topic, _admission) = hub.subscribe("slow").expect("subscriber");
        let mut receiver = topic.sender.subscribe();
        for position in 0..=TopicHub::BROADCAST_RECORDS as u64 {
            let _ = topic.sender.send((position, String::from("frame"), false));
        }
        assert!(matches!(
            receiver.recv().await,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(_))
        ));
        cancellation.cancel();
    }

    #[test]
    fn topic_open_rejects_oversize_and_non_regular_inputs() {
        let root = tempfile::tempdir().expect("topic root");
        let topics = topic_directory(root.path());
        let file = fs::File::create(topics.join("oversized.jsonl")).expect("oversized file");
        file.set_len(TopicHub::MAX_LOG_BYTES + 1)
            .expect("oversized length");
        let (hub, _, _) = topic_hub(root.path());
        assert!(hub.files.open_file("oversized").is_err());
        fs::create_dir(topics.join("directory.jsonl")).expect("non-regular topic");
        assert!(hub.files.open_file("directory").is_err());
    }

    #[test]
    fn topic_records_are_canonical_json_and_cannot_inject_sse_fields() {
        assert_eq!(
            TopicTailer::frame(7, " {\"value\":1} \r\n").expect("CRLF JSON"),
            "id: 7\ndata: {\"value\":1}\n\n"
        );
        assert_eq!(
            TopicTailer::frame(8, "{\"text\":\"event: forged\\nid: 99\\nretry: 0\"}\n")
                .expect("escaped controls remain JSON"),
            "id: 8\ndata: {\"text\":\"event: forged\\nid: 99\\nretry: 0\"}\n\n"
        );
        for invalid in [
            "\n",
            "not-json\n",
            "event: forged\n",
            "id: 99\n",
            "retry: 0\n",
            "{\"value\":1}\rforged\n",
            "{\"value\":1}",
        ] {
            assert!(
                TopicTailer::frame(9, invalid).is_err(),
                "accepted {invalid:?}"
            );
        }
        assert!(
            TopicTailer::frame(0, "1\n")
                .expect("first")
                .starts_with("id: 0\n")
        );
        assert!(
            TopicTailer::frame(1, "2\n")
                .expect("second")
                .starts_with("id: 1\n")
        );
    }

    #[test]
    fn topic_error_frames_are_terminal_topic_events() {
        let error = TopicFailure::error_frame(
            "TY_TOPIC_SLOW_SUBSCRIBER",
            "Topic subscriber was too slow.",
            "Reconnect without an explicit cursor.",
        );
        assert!(error.starts_with("event: topic-error\ndata: "));
        assert!(!error.starts_with("event: error\n"));
        assert!(!error.contains("TY2107"));
        let payload = error
            .lines()
            .find_map(|line| line.strip_prefix("data: "))
            .expect("error payload");
        let payload: serde_json::Value = serde_json::from_str(payload).expect("canonical JSON");
        assert_eq!(payload["category"], "topic");
        assert_eq!(payload["code"], "TY_TOPIC_SLOW_SUBSCRIBER");
        assert_eq!(payload["terminal"], true);
        assert!(payload["message"].is_string());
        assert!(payload["guidance"].is_string());
    }

    #[tokio::test]
    async fn topic_hub_malformed_json_is_terminal_sanitized_and_not_replayed_as_data() {
        let root = tempfile::tempdir().expect("topic root");
        let topics = topic_directory(root.path());
        fs::write(
            topics.join("hostile.jsonl"),
            "event: forged\nid: 99\nretry: 0\n",
        )
        .expect("hostile topic");
        let (hub, cancellation, producers) = topic_hub(root.path());
        let (state, admission) = hub.subscribe("hostile").expect("subscription");
        tokio::time::timeout(Duration::from_secs(1), async {
            while replay_length(&state) == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("terminal record");
        {
            let replay = state.replay.lock().expect("replay");
            assert_eq!(replay.len(), 1);
            assert!(replay[0].2, "record was not terminal");
            assert_eq!(
                replay[0].1,
                TopicFailure::error_frame(
                    "TY_TOPIC_INVALID_JSON",
                    "Topic record is not valid JSON.",
                    "Repair the NDJSON record before reconnecting.",
                )
            );
            assert!(!replay[0].1.contains("forged"));
        }
        drop(admission);
        cancellation.cancel();
        tokio::time::sleep(Duration::from_millis(20)).await;
        producers.reap_finished();
        assert_eq!(producers.len(), 0);
    }

    fn fill_replay_past_capacity(state: &TopicState) -> u64 {
        let last = TopicHub::REPLAY_RECORDS as u64 + 2;
        for position in 0..=last {
            let frame = format!("id: {position}\ndata: {position}\n\n");
            state.retain_frame(position, frame, false);
        }
        last
    }

    fn assert_evicted_cursors_are_stale(state: &TopicState) {
        let explicit_stale = topic_request("/.tachyon/topics/cursor?position=2", None);
        assert_eq!(TopicStream::requested_position(&explicit_stale), Some(2));
        assert!(state.replay_snapshot(Some(2)).cursor_gap);

        let header_stale = topic_request("/.tachyon/topics/cursor?position=99", Some("1"));
        assert_eq!(TopicStream::requested_position(&header_stale), Some(2));
        assert!(state.replay_snapshot(Some(2)).cursor_gap);
    }

    fn assert_retained_cursors_replay(state: &TopicState, oldest: u64, last: u64) {
        let covered = state.replay_snapshot(Some(oldest));
        assert_eq!(covered.resume, oldest);
        assert!(!covered.cursor_gap);
        assert_eq!(covered.frames.first().expect("oldest").0, oldest);
        assert!(
            covered
                .frames
                .windows(2)
                .all(|pair| pair[0].0 + 1 == pair[1].0)
        );

        let current_request =
            topic_request(&format!("/.tachyon/topics/cursor?position={last}"), None);
        assert_eq!(
            TopicStream::requested_position(&current_request),
            Some(last)
        );
        let current = state.replay_snapshot(Some(last));
        assert!(!current.cursor_gap);
        let positions = current
            .frames
            .iter()
            .map(|frame| frame.0)
            .collect::<Vec<_>>();
        assert_eq!(positions, [last]);

        let future_position = last + 10;
        let future_request = topic_request(
            &format!("/.tachyon/topics/cursor?position={future_position}"),
            None,
        );
        assert_eq!(
            TopicStream::requested_position(&future_request),
            Some(future_position)
        );
        let future = state.replay_snapshot(Some(future_position));
        assert!(!future.cursor_gap);
        assert!(future.frames.is_empty());
    }

    fn assert_cursorless_requests_start_at_the_floor(state: &TopicState, oldest: u64) {
        let last_event = topic_request("/.tachyon/topics/cursor", Some("2"));
        assert_eq!(TopicStream::requested_position(&last_event), Some(oldest));

        let cursorless = topic_request("/.tachyon/topics/cursor", None);
        assert_eq!(TopicStream::requested_position(&cursorless), None);
        let floor = state.replay_snapshot(None);
        assert_eq!(floor.resume, oldest);
        assert!(!floor.cursor_gap);
        assert_eq!(floor.frames.first().expect("floor").0, oldest);
    }

    async fn stale_cursor_body(cancellation: &CancellationToken) -> String {
        use tokio::io::AsyncReadExt as _;

        let (writer, mut reader) = tokio::io::duplex(1024);
        TopicStream::close_stale_cursor(writer, cancellation.clone()).await;
        let mut body = String::new();
        reader.read_to_string(&mut body).await.expect("closed body");
        body
    }

    #[tokio::test]
    async fn topic_replay_floor_rejects_only_evicted_cursors_and_closes_once() {
        let root = tempfile::tempdir().expect("topic root");
        let (hub, cancellation, _producers) = topic_hub(root.path());
        let (state, admission) = hub.subscribe("cursor").expect("subscription");
        let last = fill_replay_past_capacity(&state);
        let oldest = 3;

        assert_evicted_cursors_are_stale(&state);
        assert_retained_cursors_replay(&state, oldest, last);
        assert_cursorless_requests_start_at_the_floor(&state, oldest);

        let body = stale_cursor_body(&cancellation).await;
        assert_eq!(
            body,
            TopicFailure::error_frame(
                "TY_TOPIC_CURSOR_STALE",
                "Topic cursor is no longer available.",
                "Close this subscription and reconnect without an explicit cursor.",
            )
        );
        assert_eq!(body.matches("event: topic-error").count(), 1);
        assert_eq!(state.subscribers.load(Ordering::Acquire), 1);
        drop(admission);
        cancellation.cancel();
    }

    #[cfg(unix)]
    #[test]
    fn topic_open_never_follows_a_symlink() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().expect("topic root");
        let outside = root.path().join("outside.jsonl");
        fs::write(&outside, "{\"canary\":true}\n").expect("outside");
        let topics = topic_directory(root.path());
        symlink(&outside, topics.join("topic.jsonl")).expect("topic symlink");
        let (hub, _, _) = topic_hub(root.path());
        assert!(hub.files.open_file("topic").is_err());
    }

    #[cfg(unix)]
    fn assert_ancestor_symlink_is_rejected(outside: &Path) {
        use std::os::unix::fs::symlink;

        let linked_data = tempfile::tempdir().expect("linked project");
        symlink(outside, linked_data.path().join("data")).expect("data link");
        let error = TopicHub::new(
            topic_folder(linked_data.path()),
            CancellationToken::new(),
            ProducerTasks::new(),
        )
        .err()
        .expect("ancestor symlink rejected");
        assert!(!error.contains("sentinel"));
    }

    #[cfg(unix)]
    fn assert_root_swap_keeps_the_owned_topics(outside: &Path) {
        use std::io::Read as _;
        use std::os::unix::fs::symlink;

        let root_parent = tempfile::tempdir().expect("root parent");
        let root = root_parent.path().join("project");
        let topics = topic_directory(&root);
        fs::write(topics.join("orders.jsonl"), "{\"owned\":true}\n").expect("owned");
        let (hub, _, _) = topic_hub(&root);
        fs::rename(&root, root_parent.path().join("retained")).expect("root swap");
        fs::create_dir_all(root.join("data")).expect("planted data");
        symlink(outside, root.join(TopicHub::DIRECTORY)).expect("planted topics");
        let mut opened = hub
            .files
            .open_file("orders")
            .expect("capability open")
            .expect("owned topic");
        let mut content = String::new();
        opened.read_to_string(&mut content).expect("read owned");
        assert_eq!(content, "{\"owned\":true}\n");
        assert!(!content.contains("sentinel"));
    }

    #[cfg(unix)]
    fn assert_root_swap_keeps_missing_topics_missing(outside: &Path) {
        use std::os::unix::fs::symlink;

        let missing_parent = tempfile::tempdir().expect("missing parent");
        let missing_root = missing_parent.path().join("project");
        fs::create_dir(&missing_root).expect("missing root");
        let (missing_hub, _, _) = topic_hub(&missing_root);
        fs::rename(&missing_root, missing_parent.path().join("retained")).expect("swap missing");
        fs::create_dir_all(missing_root.join("data")).expect("planted ancestor");
        symlink(outside, missing_root.join(TopicHub::DIRECTORY)).expect("planted missing topics");
        let opened = missing_hub.files.open_file("secret").expect("safe missing");
        assert!(opened.is_none());
    }

    #[cfg(unix)]
    #[test]
    fn topic_capabilities_reject_ancestor_symlinks_and_ignore_root_swaps() {
        let outside = tempfile::tempdir().expect("outside");
        fs::write(outside.path().join("secret.jsonl"), "{\"sentinel\":true}\n").expect("sentinel");
        assert_ancestor_symlink_is_rejected(outside.path());
        assert_root_swap_keeps_the_owned_topics(outside.path());
        assert_root_swap_keeps_missing_topics_missing(outside.path());
    }

    #[tokio::test]
    async fn a_background_task_panic_is_contained_by_the_owned_runtime() {
        let mut runtime = runtime_without_plan();
        runtime.tasks.spawn(async {
            panic!("background panic canary");
        });
        tokio::task::yield_now().await;
        runtime
            .shutdown_until(tokio::time::Instant::now() + ServerRuntime::SHUTDOWN)
            .await;
        assert!(runtime.tasks.is_empty());
    }

    struct DropCanary(Arc<AtomicBool>);

    impl Drop for DropCanary {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    async fn ignore_cancellation(canary: DropCanary) {
        let _held = &canary;
        std::future::pending::<()>().await;
    }

    #[tokio::test]
    async fn abort_settlement_drops_both_task_registries_before_shutdown_returns() {
        let background_dropped = Arc::new(AtomicBool::new(false));
        let producer_dropped = Arc::new(AtomicBool::new(false));
        let mut runtime = runtime_without_plan();
        let background_canary = DropCanary(Arc::clone(&background_dropped));
        runtime.tasks.spawn(ignore_cancellation(background_canary));
        let producer_canary = DropCanary(Arc::clone(&producer_dropped));
        assert!(
            runtime
                .producers
                .spawn(ignore_cancellation(producer_canary))
        );
        tokio::task::yield_now().await;

        let started = Instant::now();
        runtime
            .shutdown_until(tokio::time::Instant::now() + ServerRuntime::ABORT_SETTLEMENT)
            .await;
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(background_dropped.load(Ordering::SeqCst));
        assert!(producer_dropped.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn draining_many_ready_tasks_yields_to_the_current_thread_timer() {
        let yielded = Arc::new(AtomicBool::new(false));
        let marker = Arc::clone(&yielded);
        let marker_task = tokio::spawn(async move {
            tokio::task::yield_now().await;
            marker.store(true, Ordering::SeqCst);
        });
        let mut runtime = runtime_without_plan();
        for _ in 0..1_024 {
            runtime.tasks.spawn(async {});
            assert!(runtime.producers.spawn(async {}));
        }
        tokio::task::yield_now().await;
        runtime
            .shutdown_until(tokio::time::Instant::now() + ServerRuntime::SHUTDOWN)
            .await;
        marker_task.await.expect("marker task");
        assert!(yielded.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn completed_producer_records_are_reaped_while_admission_remains_open() {
        let producers = ProducerTasks::new();
        let cancellation = CancellationToken::new();
        let reaper = tokio::spawn(ServerRuntime::reap_producer_tasks(
            producers.clone(),
            cancellation.clone(),
        ));
        for _ in 0..512 {
            assert!(producers.spawn(async {}));
        }
        assert!(producers.spawn(async { panic!("producer panic canary") }));
        tokio::time::timeout(Duration::from_secs(1), async {
            while producers.len() != 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("completed response tasks are continuously reaped");
        assert!(producers.spawn(async {}), "admission remains open");
        tokio::time::timeout(Duration::from_secs(1), async {
            while producers.len() != 0 {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("post-baseline task is reaped");
        cancellation.cancel();
        reaper.await.expect("producer reaper settles");
    }

    #[test]
    fn invocation_failure_events_are_correlatable_and_redacted() {
        let failure = Failure::one(diagnostic(
            2101,
            "secret-canary from process stderr",
            Some(String::from("secret-canary recovery detail")),
            None,
        ));
        for operation in ["handler", "middleware", "middleware.after"] {
            let event = InvocationLog::failure_event(
                operation,
                "0ABC123DEFG",
                Some("javascript"),
                &failure,
            );
            assert!(event.contains(r#""event":"handler.invocation_failed""#));
            assert!(event.contains(&format!(r#""operation":"{operation}""#)));
            assert!(event.contains(r#""request_id":"0ABC123DEFG""#));
            assert!(event.contains(r#""diagnostic_codes":["TY2101"]"#));
            assert!(!event.contains("runtime_family"));
            assert!(!event.contains("failure_kind"));
            assert!(!event.contains("secret-canary"));
        }
    }

    #[tokio::test]
    async fn invocation_failure_responses_are_correlatable_and_redacted() {
        for kind in ["Handler execution", "Middleware execution"] {
            let request_id = "0ABC123DEFG";
            let response = ServerResponse::invocation_failure(kind, request_id);
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            let header = response
                .headers()
                .get("x-tachyon-request-id")
                .and_then(|value| value.to_str().ok());
            assert_eq!(header, Some(request_id));
            let body = to_bytes(response.into_body(), 1024)
                .await
                .expect("redacted body");
            let body = std::str::from_utf8(&body).expect("UTF-8 body");
            assert!(body.contains(kind));
            assert!(body.contains(request_id));
            assert!(!body.contains("secret-canary"));
        }
    }

    #[test]
    fn missing_runtime_events_are_typed_and_never_disclose_runtime_paths() {
        let failure = Failure::one(diagnostic(
            2112,
            "Required configured Yon JavaScript runtime was not found.",
            Some(String::from(
                "Correct YON_JAVASCRIPT_RUNTIME or --javascript-runtime.",
            )),
            None,
        ));
        let event =
            InvocationLog::failure_event("handler", "0ABC123DEFG", Some("javascript"), &failure);
        assert!(event.contains(r#""diagnostic_codes":["TY2112"]"#));
        assert!(event.contains(r#""runtime_family":"javascript""#));
        assert!(event.contains(r#""failure_kind":"not_found""#));
        assert!(!event.contains("YON_JAVASCRIPT_RUNTIME"));
        assert!(!event.contains("configured Yon"));
    }

    async fn generated_body(files: &ServeDir, patterns: &[String], uri: &str) -> Bytes {
        let request = Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("request");
        let response = StaticFiles::serve(files, patterns, request)
            .await
            .expect("response");
        to_bytes(response.into_body(), 1024).await.expect("body")
    }

    #[tokio::test]
    async fn generated_assets_take_precedence_over_dynamic_page_patterns() {
        let root = tempfile::tempdir().expect("bundle");
        fs::create_dir_all(root.path().join("docs/_topic")).expect("dynamic directory");
        fs::write(
            root.path().join("docs/_topic/index.html"),
            "<main>Dynamic topic</main>",
        )
        .expect("dynamic page");
        fs::write(
            root.path().join("docs/client.js"),
            "document.title = 'Static client';",
        )
        .expect("static asset");
        let files = ServeDir::new(root.path()).append_index_html_on_directories(true);
        let patterns = vec![String::from("/docs/_topic")];

        let asset = generated_body(&files, &patterns, "/docs/client.js").await;
        assert_eq!(asset, "document.title = 'Static client';");
        let dynamic = generated_body(&files, &patterns, "/docs/introduction").await;
        assert_eq!(dynamic, "<main>Dynamic topic</main>");
    }

    #[tokio::test]
    async fn ephemeral_loopback_binding_reports_the_real_address() {
        let root = tempfile::tempdir().expect("project");
        write_page_project(root.path());
        let options = DevServerOptions {
            port: 0,
            ..DevServerOptions::default()
        };
        let server = DevServer::bind(root.path(), &options)
            .await
            .expect("server");
        assert!(server.address().ip().is_loopback());
        assert_ne!(server.address().port(), 0);
        let build = server.build().expect("initial build evidence");
        assert_eq!(build.route_count(), 1);
        server.run_until(async {}).await.expect("clean shutdown");
    }

    fn dynamic_route_project() -> (TempDir, Project) {
        let root = tempfile::tempdir().expect("dynamic project");
        let route = root.path().join("server/controllers/HomeController.js");
        fs::create_dir_all(route.parent().expect("route parent")).expect("route directory");
        fs::write(
            route,
            "@RequestRoute(\"/\")\n@Public\nclass HomeController {\n  @GetRoute\n  getHome() { return {}; }\n}\n",
        )
        .expect("route");
        let project = ProjectDiscovery::discover(root.path()).expect("dynamic discovery");
        (root, project)
    }

    fn static_output_project() -> (TempDir, Project) {
        let root = tempfile::tempdir().expect("static project");
        fs::create_dir_all(root.path().join("client/pages")).expect("pages");
        fs::write(
            root.path().join("client/pages/tac.html"),
            "<main>Static</main>",
        )
        .expect("static source");
        fs::create_dir_all(root.path().join("dist")).expect("dist");
        fs::write(root.path().join("dist/index.html"), "<main>Static</main>")
            .expect("static output");
        let project = ProjectDiscovery::discover(root.path()).expect("static discovery");
        (root, project)
    }

    #[tokio::test]
    async fn handler_readiness_precedes_output_and_binding_but_static_projects_need_no_runtime() {
        let missing_runtime = "/private/credentials/serve-runtime-secret-canary";
        let supervisor = Arc::new(
            HandlerSupervisor::new(HandlerSupervisorOptions {
                runtimes: HandlerRuntimePrograms {
                    javascript: PathBuf::from(missing_runtime),
                    ..HandlerRuntimePrograms::default()
                },
                ..HandlerSupervisorOptions::default()
            })
            .expect("supervisor"),
        );

        let (_dynamic_root, dynamic) = dynamic_route_project();
        let failure = DevServer::bind_project_with_supervisor(
            &dynamic,
            &unbuilt_options(),
            Arc::clone(&supervisor),
        )
        .await
        .expect_err("readiness fails before missing output is inspected");
        let rendered = failure.to_string();
        assert!(rendered.contains("TY2112"), "{rendered}");
        assert!(!rendered.contains("TY1304"), "{rendered}");
        assert!(!rendered.contains(missing_runtime), "{rendered}");

        let (_static_root, static_project) = static_output_project();
        let server = DevServer::bind_project_with_supervisor(
            &static_project,
            &unbuilt_options(),
            supervisor,
        )
        .await
        .expect("static project starts without a Yon runtime");
        drop(server);
    }

    #[tokio::test]
    async fn preview_serves_existing_output_without_rebuilding_it() {
        let root = tempfile::tempdir().expect("bundle");
        let index = root.path().join("index.html");
        fs::write(&index, "<main>Published</main>").expect("output");
        let before = fs::read(&index).expect("before");
        let options = PreviewServerOptions {
            port: 0,
            ..PreviewServerOptions::default()
        };
        let server = PreviewServer::bind(root.path(), &options)
            .await
            .expect("preview");
        assert_eq!(server.root(), root.path());
        assert_ne!(server.address().port(), 0);
        assert_eq!(fs::read(&index).expect("after"), before);
        server.run_until(async {}).await.expect("clean shutdown");
    }

    #[tokio::test]
    async fn unsafe_and_occupied_bindings_have_stable_diagnostics() {
        let root = tempfile::tempdir().expect("project");
        let unsafe_options = DevServerOptions {
            host: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            ..DevServerOptions::default()
        };
        let unsafe_failure = DevServer::bind(root.path(), &unsafe_options)
            .await
            .expect_err("unsafe bind");
        assert!(unsafe_failure.to_string().contains("TY1301"));

        write_page_project(root.path());
        let occupied = StandardTcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let options = DevServerOptions {
            port: occupied.local_addr().expect("address").port(),
            ..DevServerOptions::default()
        };
        let occupied_failure = DevServer::bind(root.path(), &options)
            .await
            .expect_err("occupied");
        assert!(occupied_failure.to_string().contains("TY1302"));
    }
}
