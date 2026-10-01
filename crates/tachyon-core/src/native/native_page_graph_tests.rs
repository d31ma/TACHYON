#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;

struct NativePageGraphCheck;

impl NativePageGraphCheck {
    fn single(
        root: &Path,
        class: &str,
        source: &str,
    ) -> Result<std::path::PathBuf, crate::Failure> {
        let path = root.join(format!("client/services/{class}.rs"));
        std::fs::create_dir_all(path.parent().expect("parent")).expect("services");
        std::fs::write(&path, source).expect("source");
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source: path,
            service: String::from("root"),
            class: class.to_owned(),
            web_assembly: false,
        };
        super::rust::stage(&[input], root, "org.tachyon.root-properties")
            .map(|source| source.expect("native source"))
    }
    const WORKER_PROVIDER: &str = r#"
#[Service]
struct CounterService {
    count: std::sync::atomic::AtomicI64,
    picker: FilePicker,
    stop: std::sync::Mutex<Option<std::sync::mpsc::Sender<()>>>,
    worker: std::sync::Mutex<Option<std::thread::JoinHandle<()>>>,
    completed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}
impl CounterService {
    fn new() -> Self {
        Self {
            count: std::sync::atomic::AtomicI64::new(0), picker: FilePicker::default(),
            stop: std::sync::Mutex::new(None), worker: std::sync::Mutex::new(None),
            completed: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }
    fn increment(&self) { self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
    fn read(&self) -> i64 { self.count.load(std::sync::atomic::Ordering::SeqCst) }
    fn available(&self) -> bool { self.picker.available() }
    #[OnStart]
    fn started(&self) -> std::result::Result<(), String> {
        let (stop, receiver) = std::sync::mpsc::channel();
        let (ready, started) = std::sync::mpsc::channel();
        let completed = self.completed.clone();
        let worker = std::thread::spawn(move || {
            ready.send(()).expect("worker ready");
            receiver.recv().expect("explicit stop signal");
            completed.store(true, std::sync::atomic::Ordering::Release);
        });
        *self.stop.lock().unwrap_or_else(|error| error.into_inner()) = Some(stop);
        *self.worker.lock().unwrap_or_else(|error| error.into_inner()) = Some(worker);
        started.recv_timeout(std::time::Duration::from_secs(2)).map_err(|_| String::from("worker startup failed"))?;
        let _ = self.picker.scope.call("probe.worker.started", Vec::new());
        Ok(())
    }
    #[OnStop]
    fn stopped(&self) -> std::result::Result<(), String> {
        let stop = self.stop.lock().unwrap_or_else(|error| error.into_inner()).take();
        if let Some(stop) = stop { stop.send(()).map_err(|_| String::from("worker signal failed"))?; }
        let worker = self.worker.lock().unwrap_or_else(|error| error.into_inner()).take();
        if let Some(worker) = worker {
            worker.join().map_err(|_| String::from("worker join failed"))?;
            assert!(self.completed.load(std::sync::atomic::Ordering::Acquire));
            let _ = self.picker.scope.call("probe.worker.joined", Vec::new());
        }
        Ok(())
    }
}
"#;
    const WORKER_CALLER: &str = r###"
struct Probe;
impl Probe {
    fn tape() -> &'static std::thread::LocalKey<core::cell::RefCell<Vec<String>>> {
        thread_local! { static VALUE: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) }; }
        &VALUE
    }
    fn count(event: &str) -> usize { Self::tape().with(|tape| tape.borrow().iter().filter(|value| value.as_str() == event).count()) }
}
fn request(service: &str, owner: &str, operation: &str) -> String {
    tac_handle(&format!(r##"{{"route":"/","service":"{service}","owner":"{owner}","op":"{operation}"}}"##))
}
fn main() {
    let prototype = PROTOTYPE;
    NativeCapabilities::register(|operation, _| {
        if operation.starts_with("probe.") { Probe::tape().with(|tape| tape.borrow_mut().push(operation.to_owned())); }
        String::from("{\"status\":\"cancelled\"}")
    });
    let writer = "00000000-0000-4000-8000-000000000001";
    let reader = "00000000-0000-4000-8000-000000000002";
    for (service, owner) in [("writer", writer), ("reader", reader)] {
        let answer = request(service, owner, "init");
        assert!(!answer.contains("error"), "{answer}");
    }
    assert_eq!(Probe::count("probe.worker.started"), if prototype { 2 } else { 1 });
    assert_eq!(Probe::count("probe.worker.joined"), 0);
    let _ = request("writer", writer, "dispose");
    assert_eq!(Probe::count("probe.worker.joined"), usize::from(prototype));
    let _ = request("writer", writer, "dispose");
    assert_eq!(Probe::count("probe.worker.joined"), usize::from(prototype), "owner disposal must not join twice");
    let _ = request("reader", reader, "dispose");
    assert_eq!(Probe::count("probe.worker.joined"), if prototype { 2 } else { 0 }, "singleton worker survives zero component owners");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    let expected = if prototype { 2 } else { 1 };
    assert_eq!(Probe::count("probe.worker.joined"), expected, "page disposal must signal and join the singleton");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    assert_eq!(Probe::count("probe.worker.joined"), expected, "repeated page disposal must not join twice");
}
"###;
    const PROVIDER: &str = r"
#[Service]
struct CounterService {
    count: std::sync::atomic::AtomicI64,
    picker: FilePicker,
}
impl CounterService {
    fn new() -> Self {
        Self { count: std::sync::atomic::AtomicI64::new(0), picker: FilePicker::default() }
    }
    fn increment(&self) { self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }
    fn read(&self) -> i64 { self.count.load(std::sync::atomic::Ordering::SeqCst) }
    fn available(&self) -> bool { self.picker.available() }
}
";
    const WRITER: &str = r"
#[Service]
#[NeedsAll]
struct WriterService { counter: std::sync::Arc<CounterService> }
impl WriterService {
    fn increment(&self) { self.counter.increment(); }
    fn identity(&self) -> i64 { std::sync::Arc::as_ptr(&self.counter) as usize as i64 }
    fn available(&self) -> bool { self.counter.available() }
}
";
    const READER: &str = r"
#[Service]
#[NeedsAll]
struct ReaderService { counter: std::sync::Arc<CounterService> }
impl ReaderService {
    fn read(&self) -> i64 { self.counter.read() }
    fn identity(&self) -> i64 { std::sync::Arc::as_ptr(&self.counter) as usize as i64 }
    fn available(&self) -> bool { self.counter.available() }
}
";
    const CALLER: &str = r###"
fn request(service: &str, owner: &str, operation: &str, member: &str) -> String {
    tac_handle(&format!(r##"{{"route":"/","service":"{service}","owner":"{owner}","op":"{operation}","name":"{member}","args":[]}}"##))
}
fn main() {
    NativeCapabilities::register(|operation, _| match operation {
        "filePicker.available" => String::from("{\"status\":\"available\"}"),
        _ => String::from("{\"status\":\"cancelled\"}"),
    });
    let writer = "00000000-0000-4000-8000-000000000001";
    let reader = "00000000-0000-4000-8000-000000000002";
    for (service, owner) in [("writer", writer), ("reader", reader)] {
        let answer = request(service, owner, "init", "");
        assert!(!answer.contains("error"), "{answer}");
    }
    assert_eq!(request("writer", writer, "call", "identity"), request("reader", reader, "call", "identity"), "two exposed roots must share the actual Arc allocation");
    let _ = request("writer", writer, "call", "increment");
    assert_eq!(request("reader", reader, "call", "read"), "{\"value\":1}");
    let _ = request("writer", writer, "dispose", "");
    assert_eq!(request("reader", reader, "call", "available"), "{\"value\":true}", "disposing the first component must not revoke the shared provider capability");
    let _ = request("reader", reader, "dispose", "");
    let _ = request("reader", reader, "init", "");
    assert_eq!(request("reader", reader, "call", "read"), "{\"value\":1}", "page singleton must survive zero temporary component leases");
    assert_eq!(request("reader", reader, "call", "available"), "{\"value\":true}");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    let _ = request("reader", reader, "init", "");
    assert_eq!(request("reader", reader, "call", "read"), "{\"value\":0}", "page exit must release the singleton");
    println!("native page singleton identity, lifetime, and capability ownership passed");
}
"###;

    fn stage(root: &Path, provider: &str) -> Result<std::path::PathBuf, crate::Failure> {
        Self::stage_roots(root, provider, [Self::WRITER, Self::READER])
    }

    fn stage_roots(
        root: &Path,
        provider: &str,
        source_roots: [&str; 2],
    ) -> Result<std::path::PathBuf, crate::Failure> {
        let services = root.join("client/services");
        std::fs::create_dir_all(&services).expect("services");
        std::fs::write(services.join("CounterService.rs"), provider).expect("provider");
        let roots = [
            ("writer", "WriterService", source_roots[0]),
            ("reader", "ReaderService", source_roots[1]),
        ];
        let inputs = roots
            .into_iter()
            .map(|(service, class, text)| {
                let source = services.join(format!("{class}.rs"));
                std::fs::write(&source, text).expect("root service");
                NativeCompanionInput {
                    snapshot_root: root.to_path_buf(),
                    language: NativeCompanion::Rust,
                    source,
                    service: service.to_owned(),
                    class: class.to_owned(),
                    web_assembly: false,
                }
            })
            .collect::<Vec<_>>();
        super::rust::stage(&inputs, root, "org.tachyon.page-graph")
            .map(|source| source.expect("native Rust source"))
    }

    async fn execute(source: &Path, caller: &str) {
        let staged = std::fs::read_to_string(source).expect("staged source");
        std::fs::write(source, staged + caller).expect("native caller");
        let executable = source.with_extension("check");
        let mut command = tokio::process::Command::new("rustc");
        command
            .args(["--edition", "2024"])
            .arg(source)
            .arg("-o")
            .arg(&executable);
        let compiled =
            crate::external_command::run(&mut command, std::time::Duration::from_mins(2), 65536)
                .await
                .expect("Rust compile");
        assert!(
            compiled.status.success(),
            "{}",
            String::from_utf8_lossy(&compiled.stderr)
        );
        let mut command = tokio::process::Command::new(&executable);
        let executed =
            crate::external_command::run(&mut command, std::time::Duration::from_secs(30), 65536)
                .await
                .expect("Rust execute");
        assert!(
            executed.status.success(),
            "{}",
            String::from_utf8_lossy(&executed.stderr)
        );
    }
}

#[tokio::test]
async fn native_rust_page_graph_shares_arc_identity_until_page_exit() {
    let root = tempfile::tempdir().expect("fixture");
    let source =
        NativePageGraphCheck::stage(root.path(), NativePageGraphCheck::PROVIDER).expect("staging");
    NativePageGraphCheck::execute(&source, NativePageGraphCheck::CALLER).await;
}

#[tokio::test]
async fn native_rust_page_graph_starts_once_and_stops_in_reverse_dependency_order() {
    let root = tempfile::tempdir().expect("fixture");
    let provider = NativePageGraphCheck::PROVIDER.to_owned()
        + r#"
impl CounterService {
    #[OnStart]
    fn started(&self) { let _ = self.picker.scope.call("probe.counter.start", Vec::new()); }
    #[OnStop]
    fn stopped(&self) { let _ = self.picker.scope.call("probe.counter.stop", Vec::new()); }
}
"#;
    let writer = NativePageGraphCheck::WRITER.to_owned()
        + r#"
impl WriterService {
    #[OnStart]
    fn started(&self) { let _ = self.counter.picker.scope.call("probe.writer.start", Vec::new()); }
    #[OnStop]
    fn stopped(&self) { let _ = self.counter.picker.scope.call("probe.writer.stop", Vec::new()); }
}
"#;
    let source = NativePageGraphCheck::stage_roots(
        root.path(),
        &provider,
        [&writer, NativePageGraphCheck::READER],
    )
    .expect("staging");
    let caller = r###"
struct Probe;
impl Probe {
    fn tape() -> &'static std::thread::LocalKey<core::cell::RefCell<Vec<String>>> {
        thread_local! { static VALUE: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) }; }
        &VALUE
    }
    fn events() -> Vec<String> { Self::tape().with(|tape| tape.borrow().clone()) }
}
fn request(service: &str, owner: &str, operation: &str) -> String {
    tac_handle(&format!(r##"{{"route":"/","service":"{service}","owner":"{owner}","op":"{operation}"}}"##))
}
fn main() {
    NativeCapabilities::register(|operation, _| {
        if operation.starts_with("probe.") { Probe::tape().with(|tape| tape.borrow_mut().push(operation.to_owned())); }
        String::from("{\"status\":\"cancelled\"}")
    });
    let writer = "00000000-0000-4000-8000-000000000001";
    let reader = "00000000-0000-4000-8000-000000000002";
    let answer = request("writer", writer, "init");
    assert!(!answer.contains("error"), "{answer}");
    let answer = request("reader", reader, "init");
    assert!(!answer.contains("error"), "{answer}");
    assert_eq!(Probe::events(), ["probe.counter.start", "probe.writer.start"]);
    let _ = request("writer", writer, "dispose");
    assert_eq!(Probe::events(), ["probe.counter.start", "probe.writer.start", "probe.writer.stop"]);
    let _ = request("reader", reader, "dispose");
    assert_eq!(Probe::events().len(), 3, "shared node remains until page exit");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    assert_eq!(Probe::events(), ["probe.counter.start", "probe.writer.start", "probe.writer.stop", "probe.counter.stop"]);
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    assert_eq!(Probe::events().len(), 4, "cleanup must happen exactly once");
}
"###;
    NativePageGraphCheck::execute(&source, caller).await;
}

#[tokio::test]
async fn native_rust_page_graph_isolates_explicit_prototype_dependencies() {
    let root = tempfile::tempdir().expect("fixture");
    let provider = NativePageGraphCheck::PROVIDER.replacen(
        "#[Service]",
        "#[Service]\n#[Lifetime({\"value\":\"prototype\"})]",
        1,
    );
    let writer = NativePageGraphCheck::WRITER
        .replace("struct WriterService { counter: std::sync::Arc<CounterService> }", "struct WriterService { counter: std::sync::Arc<CounterService>, other: std::sync::Arc<CounterService> }")
        + "\nimpl WriterService { fn same(&self) -> bool { std::sync::Arc::ptr_eq(&self.counter, &self.other) } }\n";
    let source = NativePageGraphCheck::stage_roots(
        root.path(),
        &provider,
        [&writer, NativePageGraphCheck::READER],
    )
    .expect("staging");
    let caller = NativePageGraphCheck::CALLER
        .replace("assert_eq!(request(\"writer\", writer, \"call\", \"identity\"), request(\"reader\", reader, \"call\", \"identity\"),", "assert_ne!(request(\"writer\", writer, \"call\", \"identity\"), request(\"reader\", reader, \"call\", \"identity\"),")
        .replace("\"{\\\"value\\\":1}\"", "\"{\\\"value\\\":0}\"")
        .replace("let _ = request(\"writer\", writer, \"call\", \"increment\");", "assert_eq!(request(\"writer\", writer, \"call\", \"same\"), \"{\\\"value\\\":true}\");\nlet _ = request(\"writer\", writer, \"call\", \"increment\");");
    NativePageGraphCheck::execute(&source, &caller).await;
}

#[test]
fn native_rust_page_graph_rejects_arc_aliases_that_would_lose_wire_setters() {
    let root = tempfile::tempdir().expect("fixture");
    let provider = "#[Service]\n#[derive(Default)]\nstruct CounterService { count: i64 }\nimpl CounterService { fn increment(&self) {} fn read(&self) -> i64 { self.count } fn available(&self) -> bool { false } }\n";
    let _ = NativePageGraphCheck::stage(root.path(), provider)
        .expect("private non-wire state is allowed");
    let inputs = [
        ("writer", "WriterService"),
        ("reader", "ReaderService"),
        ("counter", "CounterService"),
    ]
    .into_iter()
    .map(|(service, class)| NativeCompanionInput {
        snapshot_root: root.path().to_path_buf(),
        language: NativeCompanion::Rust,
        source: root.path().join(format!("client/services/{class}.rs")),
        service: service.to_owned(),
        class: class.to_owned(),
        web_assembly: false,
    })
    .collect::<Vec<_>>();
    let failure = super::rust::stage(&inputs, root.path(), "org.tachyon.mutable-arc")
        .expect_err("unsafe Arc alias must fail");
    assert!(
        failure.to_string().contains("writable page fields"),
        "{failure}"
    );
}

#[test]
fn native_rust_page_graph_reports_cycles_before_runtime_construction() {
    let root = tempfile::tempdir().expect("fixture");
    let provider = "#[Service]\n#[NeedsAll]\nstruct CounterService { next: std::sync::Arc<CounterService> }\nimpl CounterService { fn read(&self) -> i64 { 0 } fn increment(&self) {} fn available(&self) -> bool { false } }\n";
    let failure = NativePageGraphCheck::stage(root.path(), provider)
        .expect_err("cycle must fail before execution");
    assert!(
        failure.to_string().contains("dependency cycle"),
        "{failure}"
    );
}

#[tokio::test]
async fn native_rust_page_graph_preserves_overlapping_imports_and_explicit_aliases() {
    let root = tempfile::tempdir().expect("fixture");
    let provider = "use std::sync::{Arc, Mutex as StateMutex};\n".to_owned()
        + &NativePageGraphCheck::PROVIDER
            .replace("std::sync::atomic::AtomicI64", "StateMutex<i64>")
            .replace("std::sync::atomic::AtomicI64::new(0)", "StateMutex::new(0)")
            .replace("StateMutex<i64>::new(0)", "StateMutex::new(0)")
            .replace(
                "self.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);",
                "*self.count.lock().unwrap_or_else(|error| error.into_inner()) += 1;",
            )
            .replace(
                "self.count.load(std::sync::atomic::Ordering::SeqCst)",
                "*self.count.lock().unwrap_or_else(|error| error.into_inner())",
            );
    let writer = "use std::sync::Arc;\n".to_owned() + NativePageGraphCheck::WRITER;
    let reader = "use std::sync::{Arc, Mutex};\n".to_owned() + NativePageGraphCheck::READER;
    let source = NativePageGraphCheck::stage_roots(root.path(), &provider, [&writer, &reader])
        .expect("staging");
    NativePageGraphCheck::execute(&source, NativePageGraphCheck::CALLER).await;
}

#[tokio::test]
async fn native_rust_page_graph_does_not_infer_factories_for_explicitly_composed_services() {
    let root = tempfile::tempdir().expect("fixture");
    let services = root.path().join("client/services");
    std::fs::create_dir_all(&services).expect("services");
    std::fs::write(services.join("PrivateAdapterService.rs"), "#[Service]\nstruct PrivateAdapterService { clock: std::sync::Arc<dyn std::fmt::Debug>, endpoint: String }\nimpl PrivateAdapterService { fn new(clock: std::sync::Arc<dyn std::fmt::Debug>, endpoint: String) -> Self { Self { clock, endpoint } } }\n").expect("explicit constructor");
    let provider = NativePageGraphCheck::PROVIDER.replace("Self { count:", "let _ = PrivateAdapterService::new(std::sync::Arc::new(17_i64), String::from(\"authored endpoint\"));\nSelf { count:");
    let source =
        NativePageGraphCheck::stage(root.path(), &provider).expect("explicit composition staging");
    let staged = std::fs::read_to_string(&source).expect("source");
    assert!(staged.contains("struct PrivateAdapterService"));
    assert!(!staged.contains("fn tac_definition_PrivateAdapterService"));
    NativePageGraphCheck::execute(&source, NativePageGraphCheck::CALLER).await;
}

#[tokio::test]
async fn native_rust_page_graph_rolls_back_qualified_result_startup_failures() {
    let root = tempfile::tempdir().expect("fixture");
    let provider = NativePageGraphCheck::PROVIDER.to_owned()
        + r#"
impl CounterService {
    #[OnStart]
    fn started(&self) { let _ = self.picker.scope.call("probe.counter.start", Vec::new()); }
    #[OnStop]
    fn stopped(&self) { let _ = self.picker.scope.call("probe.counter.stop", Vec::new()); }
}
"#;
    let writer = NativePageGraphCheck::WRITER.to_owned()
        + r#"
impl WriterService {
    #[OnStart]
    fn started(&self) -> ::std::result::Result<(), String> {
        let _ = self.counter.picker.scope.call("probe.writer.start", Vec::new());
        Err(String::from("private startup detail"))
    }
    #[OnStop]
    fn stopped(&self) -> ::core::result::Result<(), String> {
        let _ = self.counter.picker.scope.call("probe.writer.stop", Vec::new());
        Ok(())
    }
}
"#;
    let source = NativePageGraphCheck::stage_roots(
        root.path(),
        &provider,
        [&writer, NativePageGraphCheck::READER],
    )
    .expect("staging");
    let caller = r###"
struct Probe;
impl Probe {
    fn tape() -> &'static std::thread::LocalKey<core::cell::RefCell<Vec<String>>> {
        thread_local! { static VALUE: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) }; }
        &VALUE
    }
    fn events() -> Vec<String> { Self::tape().with(|tape| tape.borrow().clone()) }
}
fn request(service: &str, owner: &str, operation: &str) -> String {
    tac_handle(&format!(r##"{{"route":"/","service":"{service}","owner":"{owner}","op":"{operation}"}}"##))
}
fn main() {
    NativeCapabilities::register(|operation, _| {
        if operation.starts_with("probe.") { Probe::tape().with(|tape| tape.borrow_mut().push(operation.to_owned())); }
        String::from("{\"status\":\"cancelled\"}")
    });
    let writer = "00000000-0000-4000-8000-000000000001";
    let reader = "00000000-0000-4000-8000-000000000002";
    let failed = request("writer", writer, "init");
    assert!(failed.contains("Native initializer failed"), "{failed}");
    assert!(!failed.contains("private startup detail"));
    assert_eq!(Probe::events(), ["probe.counter.start", "probe.writer.start", "probe.writer.stop", "probe.counter.stop"]);
    let answer = request("reader", reader, "init");
    assert!(!answer.contains("error"), "{answer}");
    let failed = request("writer", writer, "init");
    assert!(failed.contains("error"));
    assert_eq!(Probe::events(), ["probe.counter.start", "probe.writer.start", "probe.writer.stop", "probe.counter.stop", "probe.counter.start", "probe.writer.start", "probe.writer.stop"], "a failed owner must preserve its preexisting page provider");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    assert_eq!(Probe::events().last().map(String::as_str), Some("probe.counter.stop"));
    assert_eq!(Probe::events().len(), 8);
}

"###;
    NativePageGraphCheck::execute(&source, caller).await;
}

#[tokio::test]
async fn native_rust_page_graph_applies_properties_before_root_start() {
    let root = tempfile::tempdir().expect("fixture");
    let failure = NativePageGraphCheck::single(
        root.path(),
        "LabelService",
        "#[Service]\n#[NeedsAll]\n#[Lifetime({\"value\":\"singleton\"})]\nstruct LabelService {\nlabel: String,\nobserved: String,\n}\nimpl LabelService {\n#[OnStart]\nfn started(&mut self) { self.observed = self.label.clone(); }\n}\n",
    ).expect_err("required scalars cannot be inferred");
    assert!(
        failure.to_string().contains("required constructor input"),
        "{failure}"
    );
    let source = root.path().join("runtime-properties.rs");
    let fixture = r#"
const TAC_APPLICATION_ID: &str = "org.tachyon.runtime-properties";
#[derive(Default)]
struct LabelService { label: String, observed: String }
fn definition() -> NativeServiceDefinition<std::rc::Rc<core::cell::RefCell<LabelService>>> {
    NativeServiceDefinition {
        name: "LabelService", prototype: false,
        create: |_| Ok(std::rc::Rc::new(core::cell::RefCell::new(LabelService::default()))),
        start: |value| { let mut value = value.borrow_mut(); value.observed = value.label.clone(); Ok(()) },
        stop: |_| Ok(()),
    }
}
fn tac_service_required(_: &str) -> &'static [&'static str] { &["label"] }
fn tac_service_members(_: &str) -> Option<Vec<(&'static str, TacMember)>> {
    let activation = NativePageServices::activation()?;
    let value = activation.scope.obtain_root(&definition()).ok()?;
    let label_read = value.clone();
    let label_write = value.clone();
    let observed = value.clone();
    let scope = activation.scope.clone();
    Some(vec![
        ("label", TacMember::Field {
            read: Box::new(move || TacValue::Text(label_read.borrow().label.clone())),
            write: Some(Box::new(move |value| { if let TacValue::Text(value) = value { label_write.borrow_mut().label = value; } })),
        }),
        ("observed", TacMember::Field {
            read: Box::new(move || TacValue::Text(observed.borrow().observed.clone())), write: None,
        }),
        ("__graph_start", TacMember::Hook(false, Box::new(move || scope.start_root("LabelService", false).map_err(|_| "startup failed")))),
    ])
}
"#;
    std::fs::write(
        &source,
        fixture.to_owned() + super::rust::companion_prelude(),
    )
    .expect("emitted runtime fixture");
    let caller = r###"
fn main() {
    let one = "00000000-0000-4000-8000-000000000001";
    let two = "00000000-0000-4000-8000-000000000002";
    let first = tac_handle(&format!(r##"{{"route":"/","service":"root","owner":"{one}","op":"init","props":{{"label":"given"}}}}"##));
    assert!(!first.contains("error"), "{first}");
    let observed = tac_handle(&format!(r##"{{"route":"/","service":"root","owner":"{one}","op":"get","name":"observed"}}"##));
    assert_eq!(observed, "{\"value\":\"given\"}");
    let second = tac_handle(&format!(r##"{{"route":"/","service":"root","owner":"{two}","op":"init","props":{{"label":"second"}}}}"##));
    assert!(!second.contains("error"), "{second}");
    let observed = tac_handle(&format!(r##"{{"route":"/","service":"root","owner":"{two}","op":"get","name":"observed"}}"##));
    assert_eq!(observed, "{\"value\":\"given\"}", "page singleton startup must run only once");
    let _ = tac_handle(&format!(r##"{{"route":"/","service":"root","owner":"{one}","op":"set","name":"label","value":"changed"}}"##));
    let label = tac_handle(&format!(r##"{{"route":"/","service":"root","owner":"{two}","op":"get","name":"label"}}"##));
    assert_eq!(label, "{\"value\":\"changed\"}", "existing mutable wire setters remain available");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
}
"###;
    NativePageGraphCheck::execute(&source, caller).await;
}

#[tokio::test]
async fn native_rust_page_graph_joins_owned_workers_at_their_lifetime_boundary() {
    for prototype in [false, true] {
        let root = tempfile::tempdir().expect("fixture");
        let provider = NativePageGraphCheck::WORKER_PROVIDER.replacen(
            "#[Service]",
            if prototype {
                "#[Service]\n#[Lifetime({\"value\":\"prototype\"})]"
            } else {
                "#[Service]"
            },
            1,
        );
        let source = NativePageGraphCheck::stage(root.path(), &provider).expect("worker staging");
        let caller = NativePageGraphCheck::WORKER_CALLER
            .replace("PROTOTYPE", if prototype { "true" } else { "false" });
        NativePageGraphCheck::execute(&source, &caller).await;
    }
}
