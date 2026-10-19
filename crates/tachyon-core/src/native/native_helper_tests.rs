#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::Path;

struct NativeHelperCheck;

impl NativeHelperCheck {
    const HELPER: &'static str = "#[Helper]\nstruct StepHelper {\n    count: i64,\n}\nimpl StepHelper {\n    fn new() -> Self { Self { count: 0 } }\n    pub fn increment(&mut self) { self.count += 2; }\n    pub fn count(&self) -> i64 { self.count }\n}\n";
    const CALLER: &'static str = r###"
fn main() {
    let one = "00000000-0000-4000-8000-000000000001";
    let two = "00000000-0000-4000-8000-000000000002";
    for owner in [one, two] {
        let _ = tac_handle(&format!(r##"{{"route":"/","service":"counter","owner":"{owner}","op":"init"}}"##));
    }
    let _ = tac_handle(&format!(r##"{{"route":"/","service":"counter","owner":"{one}","op":"call","name":"increment","args":[]}}"##));
    let one_answer = tac_handle(&format!(r##"{{"route":"/","service":"counter","owner":"{one}","op":"call","name":"count","args":[]}}"##));
    let two_answer = tac_handle(&format!(r##"{{"route":"/","service":"counter","owner":"{two}","op":"call","name":"count","args":[]}}"##));
    assert_eq!(one_answer, r##"{"value":2}"##);
    assert_eq!(two_answer, r##"{"value":0}"##);
    let _ = tac_handle(&format!(r##"{{"route":"/","service":"counter","owner":"{one}","op":"dispose"}}"##));
    let _ = tac_handle(&format!(r##"{{"route":"/","service":"counter","owner":"{one}","op":"init"}}"##));
    let fresh = tac_handle(&format!(r##"{{"route":"/","service":"counter","owner":"{one}","op":"call","name":"count","args":[]}}"##));
    assert_eq!(fresh, r##"{"value":0}"##);
    println!("Rust Helper DI owns independent activation state and fresh disposal generation");
}
"###;

    fn stage(
        root: &Path,
        needs: &str,
        dependency: &str,
    ) -> Result<std::path::PathBuf, crate::Failure> {
        let helper = if dependency == "Service" {
            root.join("client/services/math/StepService.rs")
        } else {
            root.join("client/helpers/math/StepHelper.rs")
        };
        let service = root.join("client/services/ui/CounterService.rs");
        std::fs::create_dir_all(helper.parent().expect("helper directory")).expect("helpers");
        std::fs::create_dir_all(service.parent().expect("service directory")).expect("services");
        let helper_source = Self::HELPER.replace("Helper]", &format!("{dependency}]"));
        let helper_source = if dependency == "Service" {
            helper_source.replace("StepHelper", "StepService")
        } else {
            helper_source
        };
        std::fs::write(&helper, helper_source).expect("dependency source");
        let (role, field) = match needs {
            "Needs" => (
                "#[Service]\n#[Lifetime({\"value\":\"prototype\"})]",
                "#[Needs]\n    step_helper: StepHelper,",
            ),
            _ => ("#[Service]\n#[NeedsAll]", "step_helper: StepHelper,"),
        };
        let source = format!(
            "{role}\nstruct CounterService {{\n    {field}\n}}\nimpl CounterService {{\n    pub fn increment(&mut self) {{ self.step_helper.increment(); }}\n    pub fn count(&self) -> i64 {{ self.step_helper.count() }}\n}}\n"
        );
        let source = if dependency == "Service" {
            source.replace("StepHelper", "StepService")
        } else {
            source
        };
        std::fs::write(&service, source).expect("service source");
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Rust,
            source: service,
            service: String::from("counter"),
            class: String::from("CounterService"),
            web_assembly: false,
            application_provider: false,
        };
        super::rust::stage(&[input], root, "org.tachyon.helper")
            .map(|source| source.expect("staged source"))
    }

    async fn execute(needs: &str, dependency: &str) {
        let root = tempfile::tempdir().expect("fixture");
        let source = Self::stage(root.path(), needs, dependency).expect("native Rust staging");
        let staged = std::fs::read_to_string(&source).expect("staged source");
        assert!(!staged.contains("#[Helper]"));
        std::fs::write(&source, staged + Self::CALLER).expect("actual native caller");
        let executable = root.path().join("helper-check");
        let mut command = tokio::process::Command::new("rustc");
        command
            .args(["--edition", "2024"])
            .arg(&source)
            .arg("-o")
            .arg(&executable);
        let build =
            crate::external_command::run(&mut command, std::time::Duration::from_mins(2), 65536)
                .await
                .expect("Rust compile");
        assert!(
            build.status.success(),
            "{}",
            String::from_utf8_lossy(&build.stderr)
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
        assert!(String::from_utf8_lossy(&executed.stdout).contains("independent activation state"));
    }
}

#[tokio::test]
async fn a_compiled_rust_helper_is_injected_with_explicit_needs() {
    NativeHelperCheck::execute("Needs", "Helper").await;
}

#[tokio::test]
async fn a_compiled_rust_helper_is_injected_with_needs_all() {
    NativeHelperCheck::execute("NeedsAll", "Helper").await;
}

#[test]
fn a_native_rust_by_value_service_dependency_requires_arc_with_explicit_needs() {
    let root = tempfile::tempdir().expect("fixture");
    let failure = NativeHelperCheck::stage(root.path(), "Needs", "Service")
        .expect_err("by-value Service cannot share page identity");
    assert!(
        failure.to_string().contains("requires Arc<Service>"),
        "{failure}"
    );
}

#[test]
fn a_native_rust_by_value_service_dependency_requires_arc_with_needs_all() {
    let root = tempfile::tempdir().expect("fixture");
    let failure = NativeHelperCheck::stage(root.path(), "NeedsAll", "Service")
        .expect_err("by-value Service cannot share page identity");
    assert!(
        failure.to_string().contains("requires Arc<Service>"),
        "{failure}"
    );
}

#[tokio::test]
async fn a_compiled_rust_arc_service_dependency_matches_the_monitor_repro() {
    let root = tempfile::tempdir().expect("fixture");
    let services = root.path().join("client/services");
    std::fs::create_dir_all(&services).expect("services");
    std::fs::write(services.join("CounterService.rs"), "#[Service]\n#[derive(Default)]\nstruct CounterService {}\nimpl CounterService {\n    pub fn read(&self) -> i64 { 1 }\n}\n").expect("provider");
    let source = services.join("SnapshotReaderService.rs");
    std::fs::write(&source, "#[Service]\n#[NeedsAll]\nstruct SnapshotReaderService {\n    counter: std::sync::Arc<CounterService>,\n}\nimpl SnapshotReaderService {\n    pub fn read(&self) -> i64 { self.counter.read() }\n}\n").expect("consumer");
    let input = NativeCompanionInput {
        snapshot_root: root.path().to_path_buf(),
        language: NativeCompanion::Rust,
        source,
        service: String::from("reader"),
        class: String::from("SnapshotReaderService"),
        web_assembly: false,
        application_provider: false,
    };
    let staged = super::rust::stage(&[input], root.path(), "org.tachyon.monitor.repro")
        .expect("staging")
        .expect("source");
    let mut text = std::fs::read_to_string(&staged).expect("read staged");
    text.push_str(r###"
fn main() {
    let owner = "00000000-0000-4000-8000-000000000001";
    let init = tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{owner}","op":"init"}}"##));
    assert!(!init.contains("error"), "{init}");
    let read = tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{owner}","op":"call","name":"read","args":[]}}"##));
    assert_eq!(read, r##"{"value":1}"##);
    println!("actual monitor Arc service repro executed");
}
"###);
    std::fs::write(&staged, text).expect("caller");
    let executable = root.path().join("monitor-repro");
    let mut compiler = tokio::process::Command::new("rustc");
    compiler
        .args(["--edition", "2024"])
        .arg(&staged)
        .arg("-o")
        .arg(&executable);
    let build =
        crate::external_command::run(&mut compiler, std::time::Duration::from_mins(2), 65536)
            .await
            .expect("compile");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut caller = tokio::process::Command::new(&executable);
    let executed =
        crate::external_command::run(&mut caller, std::time::Duration::from_secs(30), 65536)
            .await
            .expect("execute");
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
}

#[tokio::test]
async fn a_compiled_rust_arc_service_dependency_is_shared_within_its_owner() {
    let root = tempfile::tempdir().expect("fixture");
    let services = root.path().join("client/services");
    std::fs::create_dir_all(&services).expect("services");
    std::fs::write(services.join("CounterService.rs"), "#[Service]\n#[Lifetime({\"value\":\"prototype\"})]\n#[derive(Default)]\nstruct CounterService { count: std::sync::atomic::AtomicI64 }\n").expect("counter");
    let reader = services.join("ReaderService.rs");
    std::fs::write(&reader, "#[Service]\n#[Lifetime({\"value\":\"prototype\"})]\nstruct ReaderService {\n    #[Needs]\n    first: std::sync::Arc<CounterService>,\n    #[Needs]\n    second: std::sync::Arc<CounterService>,\n}\nimpl ReaderService {\n    pub fn same(&self) -> bool { std::sync::Arc::ptr_eq(&self.first, &self.second) }\n    pub fn increment(&self) { self.first.count.fetch_add(1, std::sync::atomic::Ordering::SeqCst); }\n    pub fn count(&self) -> i64 { self.second.count.load(std::sync::atomic::Ordering::SeqCst) }\n}\n").expect("reader");
    let input = NativeCompanionInput {
        snapshot_root: root.path().to_path_buf(),
        language: NativeCompanion::Rust,
        source: reader,
        service: String::from("reader"),
        class: String::from("ReaderService"),
        web_assembly: false,
        application_provider: false,
    };
    let file = super::rust::stage(&[input], root.path(), "org.tachyon.shared.providers")
        .expect("staging")
        .expect("source");
    let mut text = std::fs::read_to_string(&file).expect("staged");
    text.push_str(r###"
fn main() {
    for owner in ["00000000-0000-4000-8000-000000000001", "00000000-0000-4000-8000-000000000002"] {
        let initialized = tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{owner}","op":"init"}}"##));
        assert!(!initialized.contains("error"), "{initialized}");
        let same = tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{owner}","op":"call","name":"same","args":[]}}"##));
        assert_eq!(same, r##"{"value":true}"##);
    }
    let one = "00000000-0000-4000-8000-000000000001";
    let two = "00000000-0000-4000-8000-000000000002";
    let _ = tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{one}","op":"call","name":"increment","args":[]}}"##));
    let count = tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{one}","op":"call","name":"count","args":[]}}"##));
    assert_eq!(count, r##"{"value":1}"##);
    let other = tac_handle(&format!(r##"{{"route":"/","service":"reader","owner":"{two}","op":"call","name":"count","args":[]}}"##));
    assert_eq!(other, r##"{"value":0}"##);
}
"###);
    std::fs::write(&file, text).expect("caller");
    let executable = root.path().join("shared");
    let mut command = tokio::process::Command::new("rustc");
    command
        .args(["--edition", "2024"])
        .arg(&file)
        .arg("-o")
        .arg(&executable);
    let build =
        crate::external_command::run(&mut command, std::time::Duration::from_mins(2), 65536)
            .await
            .expect("compile");
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let mut command = tokio::process::Command::new(&executable);
    let executed =
        crate::external_command::run(&mut command, std::time::Duration::from_secs(30), 65536)
            .await
            .expect("execute");
    assert!(
        executed.status.success(),
        "{}",
        String::from_utf8_lossy(&executed.stderr)
    );
}
