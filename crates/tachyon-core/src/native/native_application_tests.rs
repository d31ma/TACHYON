#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};

pub(super) struct ApplicationLifetimeTests;

impl ApplicationLifetimeTests {
    pub(super) const COLLECTOR: &str = r#"
#[Service]
#[Lifetime({"value":"application"})]
struct CollectorService {
    count: std::sync::atomic::AtomicI64,
    stop: std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>,
    worker: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
}
impl CollectorService {
    fn new() -> Self {
        Probe::record("created");
        Self { count: std::sync::atomic::AtomicI64::new(0), stop: std::sync::Mutex::new(None), worker: std::sync::Mutex::new(None) }
    }
    pub fn read(&self) -> i64 { self.count.load(std::sync::atomic::Ordering::SeqCst) }
    pub fn increment(&self) { self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
    #[OnStart]
    pub fn started(&self) -> Result<(), String> {
        let (stop, receiver) = std::sync::mpsc::channel();
        let (ready, started) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || { ready.send(()).unwrap(); receiver.recv().unwrap(); });
        *self.stop.lock().unwrap() = Some(stop);
        *self.worker.lock().unwrap() = Some(worker);
        started.recv_timeout(std::time::Duration::from_secs(2)).map_err(|_| String::from("worker start failed"))?;
        Probe::record("started");
        Ok(())
    }
    #[OnStop]
    pub fn stopped(&self) {
        if let Some(stop) = self.stop.lock().unwrap().take() { stop.send(()).unwrap(); }
        if let Some(worker) = self.worker.lock().unwrap().take() { worker.join().unwrap(); Probe::record("joined"); }
        Probe::record("stopped");
    }
}
"#;
    pub(super) const READER: &str = r"
#[Service]
#[NeedsAll]
struct ReaderService { collector: std::sync::Arc<CollectorService>, picker: FilePicker }
impl ReaderService {
    pub fn read(&self) -> i64 { self.collector.read() }
    pub fn increment(&self) { self.collector.increment(); }
    pub fn available(&self) -> bool { self.picker.available() }
}
";
    pub(super) const PROBE: &str = r###"
struct Probe;
impl Probe {
    fn tape() -> &'static std::thread::LocalKey<core::cell::RefCell<Vec<String>>> {
        thread_local! { static VALUE: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) }; }
        &VALUE
    }
    fn record(value: &str) { Self::tape().with(|tape| tape.borrow_mut().push(value.to_owned())); }
    fn count(value: &str) -> usize { Self::tape().with(|tape| tape.borrow().iter().filter(|event| event.as_str() == value).count()) }
}
fn request(owner: &str, operation: &str, method: &str) -> String {
    tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{owner}","op":"{operation}","name":"{method}","args":[]}}"##))
}
"###;

    fn bridge(root: &Path, lifetime: &str) {
        let services = root.join("client/services/native/providers");
        let helpers = root.join("client/delegates/native/composition");
        std::fs::create_dir_all(&services).expect("provider directory");
        std::fs::create_dir_all(&helpers).expect("helper directory");
        let provider = format!(
            "#[Service]\n#[Lifetime({{\"value\":\"{lifetime}\"}})]\n#[derive(Default)]\nstruct SharedService {{}}\nimpl SharedService {{ pub fn read(&self) -> i64 {{ 42 }} }}"
        );
        std::fs::write(services.join("SharedService.rs"), provider).expect("provider");
        std::fs::write(helpers.join("BridgeDelegate.rs"), "#[Delegate]\nstruct BridgeDelegate { shared: std::sync::Arc<SharedService> }\nimpl BridgeDelegate { fn new(shared: std::sync::Arc<SharedService>) -> Self { Self { shared } } pub fn read(&self) -> i64 { self.shared.read() } }").expect("owned helper");
    }

    const INDIRECT_COLLECTOR: &str = "#[Service]\n#[Lifetime({\"value\":\"application\"})]\nstruct CollectorService { bridge: BridgeDelegate }\nimpl CollectorService { fn new(bridge: BridgeDelegate) -> Self { Self { bridge } } pub fn read(&self) -> i64 { self.bridge.read() } }";

    pub(super) fn stage(
        root: &Path,
        collector: &str,
        reader: Option<&str>,
    ) -> Result<PathBuf, crate::Failure> {
        let services = root.join("client/services/native/collectors");
        std::fs::create_dir_all(&services).expect("services");
        let source = services.join("CollectorService.rs");
        std::fs::write(&source, collector).expect("collector");
        let mut inputs = vec![NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source,
            service: String::from("collector"),
            class: String::from("CollectorService"),
            web_assembly: false,
            application_provider: true,
        }];
        if let Some(reader) = reader {
            let source = root.join("client/services/ReaderService.rs");
            std::fs::write(&source, reader).expect("reader");
            inputs.push(NativeCompanionInput {
                snapshot_root: root.to_path_buf(),
                language: NativeCompanion::Rust,
                source,
                service: String::from("reader"),
                class: String::from("ReaderService"),
                web_assembly: false,
                application_provider: false,
            });
        }
        super::rust::stage(&inputs, root, "org.tachyon.application-scope")
            .map(|source| source.expect("native Rust source"))
    }
}

#[test]
fn native_application_lifetime_is_located_and_target_scoped() {
    use tachyon_contracts::NativeTarget;
    let path = Path::new("client/services/native/CollectorService.rs");
    let roles = crate::behavior::ServiceRoles::read(path, ApplicationLifetimeTests::COLLECTOR)
        .expect("roles");
    assert!(!roles.prototype);
    assert!(roles.application.is_some());
    for target in [
        NativeTarget::Macos,
        NativeTarget::Linux,
        NativeTarget::Windows,
        NativeTarget::Ios,
        NativeTarget::Android,
    ] {
        crate::behavior::NativeApplicationLifetime::target(path, roles.application, Some(target))
            .expect("every native host owns the application lifetime");
    }
    let failure = crate::behavior::NativeApplicationLifetime::target(path, roles.application, None)
        .expect_err("a browser page has no application lifetime");
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), 1406, "{failure}");
    assert!(
        diagnostic
            .message
            .contains("use a page or singleton Service"),
        "{failure}"
    );
    assert_eq!(diagnostic.spans[0].file, path.to_string_lossy());
    assert_eq!(
        diagnostic.spans[0].start,
        u64::try_from(roles.application.expect("authored marker")).expect("marker offset")
    );
    for path in [
        "server/services/CollectorService.rs",
        "client/delegates/CollectorService.rs",
    ] {
        assert!(
            crate::behavior::ServiceRoles::read(
                Path::new(path),
                ApplicationLifetimeTests::COLLECTOR
            )
            .is_err(),
            "{path}"
        );
    }
    let csharp = "[Service]\n[Lifetime({\"value\":\"application\"})]\nclass CollectorService {}";
    assert!(
        crate::behavior::ServiceRoles::read(
            Path::new("client/services/CollectorService.cs"),
            csharp
        )
        .is_err()
    );
}

#[test]
fn native_application_lifetime_refuses_page_capture_and_direct_binding() {
    for provider in [
        "FilePicker",
        "WindowControls",
        "Clock",
        "DocumentTitle",
        "ApplicationEventPublisher",
    ] {
        let root = tempfile::tempdir().expect("fixture");
        let collector = format!(
            "#[Service]\n#[Lifetime({{\"value\":\"application\"}})]\n#[derive(Default)]\nstruct CollectorService {{ port: {provider} }}"
        );
        let failure = ApplicationLifetimeTests::stage(root.path(), &collector, None)
            .expect_err("page port refusal");
        let diagnostic = &failure.diagnostics()[0];
        let expected = if provider == "DocumentTitle" {
            1317
        } else {
            1406
        };
        assert_eq!(diagnostic.code.number(), expected, "{provider}: {failure}");
        assert!(
            diagnostic.spans[0].file.ends_with("CollectorService.rs"),
            "{failure}"
        );
        let message = if provider == "DocumentTitle" {
            "no supported provider for this linked native platform"
        } else {
            "application Service cannot"
        };
        assert!(
            diagnostic.message.contains(message),
            "{provider}: {failure}"
        );
    }
    let root = tempfile::tempdir().expect("fixture");
    let collector = ApplicationLifetimeTests::COLLECTOR;
    let source = ApplicationLifetimeTests::stage(root.path(), collector, None)
        .expect("provider-only source");
    assert!(
        std::fs::read_to_string(source)
            .expect("source")
            .contains("tac_application_providers")
    );
    let input = NativeCompanionInput {
        snapshot_root: root.path().to_path_buf(),
        language: NativeCompanion::Rust,
        source: root
            .path()
            .join("client/services/native/collectors/CollectorService.rs"),
        service: String::from("collector"),
        class: String::from("CollectorService"),
        web_assembly: false,
        application_provider: false,
    };
    let failure = super::rust::stage(&[input], root.path(), "org.tachyon.direct-app")
        .expect_err("direct binding refusal");
    assert!(
        format!("{failure:?}").contains("directly view-bound"),
        "{failure:?}"
    );
}

#[test]
fn native_application_lifetime_refuses_shorter_managed_dependencies_and_props() {
    let root = tempfile::tempdir().expect("fixture");
    let services = root.path().join("client/services");
    std::fs::create_dir_all(&services).expect("services");
    std::fs::write(
        services.join("PageService.rs"),
        "#[Service]\n#[derive(Default)]\nstruct PageService {}",
    )
    .expect("page provider");
    let collector = "#[Service]\n#[Lifetime({\"value\":\"application\"})]\n#[NeedsAll]\nstruct CollectorService { page: std::sync::Arc<PageService> }";
    let failure = ApplicationLifetimeTests::stage(root.path(), collector, None)
        .expect_err("page dependency refusal");
    assert!(
        format!("{failure:?}").contains("cannot capture a page"),
        "{failure:?}"
    );
    let collector = "#[Service]\n#[Lifetime({\"value\":\"application\"})]\n#[NeedsAll]\nstruct CollectorService { label: String }";
    let failure = ApplicationLifetimeTests::stage(root.path(), collector, None)
        .expect_err("view prop refusal");
    assert_eq!(failure.diagnostics()[0].code.number(), 1406, "{failure}");
    assert!(
        failure.diagnostics()[0]
            .message
            .contains("no page properties"),
        "{failure}"
    );
    assert!(
        failure.diagnostics()[0].spans[0]
            .file
            .ends_with("CollectorService.rs"),
        "{failure}"
    );
}

#[test]
fn native_application_lifetime_refuses_indirect_helper_capture_of_shorter_services() {
    for lifetime in ["singleton", "prototype"] {
        let root = tempfile::tempdir().expect("fixture");
        ApplicationLifetimeTests::bridge(root.path(), lifetime);
        let failure = ApplicationLifetimeTests::stage(
            root.path(),
            ApplicationLifetimeTests::INDIRECT_COLLECTOR,
            None,
        )
        .expect_err("application cannot capture a shorter Service through an owned Delegate");
        let diagnostic = &failure.diagnostics()[0];
        assert_eq!(diagnostic.code.number(), 1406, "{lifetime}: {failure}");
        assert!(
            diagnostic
                .message
                .contains("application Service cannot capture a page or prototype Service"),
            "{lifetime}: {failure}"
        );
        assert!(
            diagnostic.spans[0].file.ends_with("CollectorService.rs"),
            "{failure}"
        );
    }
}

#[tokio::test]
async fn native_application_lifetime_preserves_indirect_application_helper_composition() {
    let root = tempfile::tempdir().expect("fixture");
    ApplicationLifetimeTests::bridge(root.path(), "application");
    let reader = "#[Service]\n#[NeedsAll]\nstruct ReaderService { collector: std::sync::Arc<CollectorService> }\nimpl ReaderService { pub fn read(&self) -> i64 { self.collector.read() } }";
    let source = ApplicationLifetimeTests::stage(
        root.path(),
        ApplicationLifetimeTests::INDIRECT_COLLECTOR,
        Some(reader),
    )
    .expect("application-compatible Delegate dependency");
    let caller = r#"
fn main() {
    assert!(tac_native_application_start());
    let owner = "00000000-0000-4000-8000-000000000001";
    assert!(!request(owner, "init", "").contains("error"));
    assert_eq!(request(owner, "call", "read"), "{\"value\":42}", "the owned Delegate retains the actual injected application provider");
    let page = NativeApplicationServices::page().unwrap();
    assert_eq!(page.borrow().order.len(), 2, "one application node for the provider and one for its owned Delegate consumer");
    assert!(tac_native_application_start());
    assert_eq!(page.borrow().order.len(), 2, "ready application providers retain their identity");
    tac_native_application_shutdown();
    assert!(page.borrow().order.is_empty());
    assert!(tac_native_application_start(), "a cleanly stopped application starts again");
    assert!(page.borrow().order.is_empty(), "the new start does not reuse the stopped page");
    tac_native_application_shutdown();
}
"#;
    super::native_page_graph_tests::NativePageGraphCheck::execute(
        &source,
        &(ApplicationLifetimeTests::PROBE.to_owned() + caller),
    )
    .await;
}

#[tokio::test]
async fn native_application_lifetime_survives_help_and_joins_only_on_quit() {
    let root = tempfile::tempdir().expect("fixture");
    let source = ApplicationLifetimeTests::stage(
        root.path(),
        ApplicationLifetimeTests::COLLECTOR,
        Some(ApplicationLifetimeTests::READER),
    )
    .expect("stage");
    let caller = r#"
fn main() {
    NativeCapabilities::register(|_, _| String::from("{\"status\":\"available\"}"));
    assert!(tac_native_application_start(), "host starts before any reader exists, including first Help route");
    assert!(tac_native_application_start());
    assert_eq!(Probe::count("created"), 1);
    assert_eq!(Probe::count("started"), 1);
    std::thread::spawn(|| {
        assert!(!tac_native_application_start(), "another thread cannot start a second collector");
        assert!(NativeApplicationServices::page().is_err());
        tac_native_application_shutdown();
    }).join().unwrap();
    assert_eq!(Probe::count("stopped"), 0, "wrong-thread shutdown cannot stop the host collector");
    let old = "00000000-0000-4000-8000-000000000001";
    assert!(!request(old, "init", "").contains("error"));
    assert_eq!(request(old, "call", "read"), "{\"value\":0}");
    assert_eq!(request(old, "call", "available"), "{\"value\":true}");
    let _ = request(old, "call", "increment");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    assert!(request(old, "call", "read").contains("error"));
    assert_eq!(Probe::count("stopped"), 0, "Help navigation does not stop the collector");
    let fresh = "00000000-0000-4000-8000-000000000002";
    assert!(!request(fresh, "init", "").contains("error"));
    assert_eq!(request(fresh, "call", "read"), "{\"value\":1}");
    assert_eq!(Probe::count("created"), 1);
    let forbidden = tac_handle("{\"op\":\"application-start\"}");
    assert!(forbidden.contains("error"), "page bridge has no startup authority");
    tac_native_application_shutdown();
    tac_native_application_shutdown();
    assert_eq!(Probe::count("joined"), 1);
    assert_eq!(Probe::count("stopped"), 1);
    assert!(request(fresh, "call", "read").contains("error"));
    assert!(tac_native_application_start(), "a host may start a cleanly stopped application again (a new Android activity)");
    assert_eq!(Probe::count("created"), 2, "the new start creates new application services");
    let again = "00000000-0000-4000-8000-000000000003";
    assert!(!request(again, "init", "").contains("error"));
    assert_eq!(request(again, "call", "read"), "{\"value\":0}", "nothing carries over from the stopped instance");
    tac_native_application_shutdown();
    assert_eq!(Probe::count("stopped"), 2);
    println!("application worker continuity and exactly-once joined shutdown passed");
}
"#;
    super::native_page_graph_tests::NativePageGraphCheck::execute(
        &source,
        &(ApplicationLifetimeTests::PROBE.to_owned() + caller),
    )
    .await;
}

#[tokio::test]
async fn native_application_lifetime_retires_after_failed_start_and_constructor_panic() {
    for constructor_panic in [false, true] {
        let root = tempfile::tempdir().expect("fixture");
        let services = root.path().join("client/services");
        std::fs::create_dir_all(&services).expect("services");
        let constructor = if constructor_panic {
            "panic!(\"authored constructor failure\")"
        } else {
            "Self {}"
        };
        let failure = format!(
            r#"
#[Service]
#[Lifetime({{"value":"application"}})]
struct FailureService {{}}
impl FailureService {{
    fn new() -> Self {{ {constructor} }}
    #[OnStart]
    pub fn start(&self) -> Result<(), String> {{ Err(String::from("authored startup failure")) }}
    #[OnStop]
    pub fn stop(&self) {{ Probe::record("failure.stopped"); panic!("authored stop panic"); }}
}}
"#
        );
        std::fs::write(services.join("FailureService.rs"), failure).expect("failure provider");
        let source =
            ApplicationLifetimeTests::stage(root.path(), ApplicationLifetimeTests::COLLECTOR, None)
                .expect("stage");
        let caller = r#"
fn main() {
    assert!(!tac_native_application_start());
    assert_eq!(Probe::count("joined"), 1, "a failed or panicking later provider must still join the earlier worker");
    assert_eq!(Probe::count("stopped"), 1);
    assert!(!tac_native_application_start());
    tac_native_application_shutdown();
    assert_eq!(Probe::count("stopped"), 1);
}
"#;
        super::native_page_graph_tests::NativePageGraphCheck::execute(
            &source,
            &(ApplicationLifetimeTests::PROBE.to_owned() + caller),
        )
        .await;
    }
}

#[test]
fn native_application_lifetime_host_startup_and_shutdown_are_not_page_operations() {
    let application = super::config::NativeApplication {
        name: String::from("Lifecycle"),
        executable_name: String::from("Lifecycle"),
        application_id: String::from("org.tachyon.lifecycle"),
        version: String::from("1.0.0"),
        build: 1,
        entry_route: String::from("/"),
        icons: Vec::new(),
        window: super::config::WindowConfiguration::default(),
        native_capabilities: Vec::new(),
        menus: super::native_menu_config::NativeMenuConfig::default(),
        rust: crate::native::native_rust_project::NativeRustProject::default(),
        outgoing: crate::native::native_outgoing_hosts::NativeOutgoingHosts::default(),
        permissions: crate::native::browser_api_permissions::BrowserApiPermissions::default(),
        minimum_system: crate::native::minimum_system::MinimumSystem::default(),
        description: String::new(),
        tray: None,
        links: crate::native::native_links::NativeLinks::default(),
    };
    let index = super::routes::NativeRouteIndex {
        contract_version: 2,
        entry_route: String::from("/"),
        entry_document: String::from("index.html"),
        routes: vec![super::routes::NativeRouteIndexEntry {
            route: String::from("/"),
            document: String::from("index.html"),
        }],
    };
    let source = super::macos::swift_source(&application, "tacRustInvoke(payload)", &index);
    assert!(
        source
            .find("guard tacRustNativeApplicationStart()")
            .expect("startup")
            < source
                .find("let configuration = WKWebViewConfiguration()")
                .expect("view")
    );
    let navigation = source
        .split("didStartProvisionalNavigation")
        .nth(1)
        .expect("navigation")
        .split("func webView")
        .next()
        .expect("body");
    assert!(!navigation.contains("tacRustNativeApplicationShutdown"));
    let termination = source
        .split("applicationWillTerminate")
        .nth(1)
        .expect("quit");
    assert!(termination.contains("tacRustNativeApplicationShutdown()"));
    assert!(!source.contains("\"application-start\", \"init\""));
}
