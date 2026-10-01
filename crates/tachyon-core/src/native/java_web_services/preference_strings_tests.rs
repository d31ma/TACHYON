#![allow(clippy::expect_used)]

use super::JavaWebServices;
use super::java_models::JavaModels;
use super::java_service::JavaService;
use super::java_service_table::JavaServiceTable;
use super::java_web_apis::JavaWebApis;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

struct PreferenceFixture;

impl PreferenceFixture {
    const SOURCE: &'static str = "@Service\nclass ChoiceService {\n @Needs\n private final PreferenceStrings preferences;\n String read() { return preferences.read(\"tachyon.platform\"); }\n boolean write(String value) { return preferences.write(\"tachyon.platform\", value); }\n}\n";
    const HOST: &'static str = r#"final class TacHost {
    static int calls;
    static String value;
    static void emit(String payload) {}
    static void pause(double milliseconds) {}
    static String storage(String payload) { return "{\"value\":null}"; }
    static String web(String payload) {
        calls++;
        java.util.Map<?, ?> request = (java.util.Map<?, ?>) TacJsonReader.read(payload);
        java.util.List<?> args = (java.util.List<?>) request.get("args");
        if (!"tachyon.platform".equals(args.get(0))) throw new AssertionError("raw key changed");
        if ("write".equals(request.get("call"))) {
            value = (String) args.get(1);
            return "{\"value\":true}";
        }
        return "{\"value\":" + TacJsonWriter.write(value) + "}";
    }
}
"#;
    const DRIVER: &'static str = r#"final class PreferenceMain {
    public static void main(String[] ignored) {
        TacActivation first = new TacActivation("/", "choice", "first");
        TacMethod retainedRead = (TacMethod) first.members.get("read");
        TacMethod retainedWrite = (TacMethod) first.members.get("write");
        require(Boolean.TRUE.equals(retainedWrite.invoke.apply(java.util.List.of("macOS"))), "live write");
        require("macOS".equals(retainedRead.invoke.apply(java.util.List.of())), "raw read");
        first.dispose();
        TacActivation second = new TacActivation("/", "choice", "second");
        int before = TacHost.calls;
        require(retainedRead.invoke.apply(java.util.List.of()) == null, "disposed read");
        require(Boolean.FALSE.equals(retainedWrite.invoke.apply(java.util.List.of("stale"))), "disposed write");
        require(before == TacHost.calls, "a retained disposed provider never calls the host");
        require(Boolean.TRUE.equals(((TacMethod) second.members.get("write")).invoke.apply(java.util.List.of("Linux"))), "new owner write");
        require("Linux".equals(TacHost.value), "new owner active");
        require(retainedRead.invoke.apply(java.util.List.of()) == null, "old owner does not follow the new one");
        second.dispose();
        System.out.println("preference-ownership-qualified");
    }
    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
}
"#;

    fn prepared(root: &Path) -> (JavaService, super::java_value_type::JavaValueType) {
        Self::prepared_source(root, Self::SOURCE)
    }

    fn prepared_source(
        root: &Path,
        source: &str,
    ) -> (JavaService, super::java_value_type::JavaValueType) {
        let path = root.join("client/services/ChoiceService.java");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("service");
        let models = JavaModels::read(root, source).expect("models");
        let values = models.values();
        let authored = JavaService {
            instance: String::from("choice"),
            class: String::from("ChoiceService"),
            members: Vec::new(),
            required: Vec::new(),
            providers: Vec::new(),
            path: path.strip_prefix(root).expect("relative").to_path_buf(),
            source: String::from(source),
        };
        let service = JavaWebServices::service(root, authored, 0, &values, &models)
            .expect("production Java lowering");
        (service, values)
    }

    fn staged(root: &Path) -> Vec<std::path::PathBuf> {
        let (service, values) = Self::prepared(root);
        let mut files = JavaWebServices::prelude(std::slice::from_ref(&service), &values);
        files.retain(|(path, _)| {
            path.file_name()
                .is_none_or(|name| name != "TacHost.java" && name != "TacServices.java")
        });
        let models = JavaModels::read(root, Self::SOURCE).expect("models");
        files.extend(models.sources(&values).expect("model codecs"));
        files.extend(JavaWebApis::clients(Path::new("tachyon"), Self::SOURCE));
        files.extend([
            (
                std::path::PathBuf::from("ChoiceService.java"),
                service.source,
            ),
            (
                std::path::PathBuf::from("TacHost.java"),
                String::from(Self::HOST),
            ),
            (
                std::path::PathBuf::from("PreferenceMain.java"),
                String::from(Self::DRIVER),
            ),
        ]);
        files
            .into_iter()
            .map(|(relative, source)| {
                let path = root.join(relative);
                std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
                std::fs::write(&path, source).expect("source");
                path
            })
            .collect()
    }

    fn execute(command: &mut Command) -> String {
        let output = crate::external_command::run_sync(command, Duration::from_mins(1), 262_144)
            .expect("supervised JDK");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
fn a_java_needs_preference_field_has_a_typed_constructor_and_is_not_a_page_member() {
    let root = tempfile::tempdir().expect("project");
    let (service, values) = PreferenceFixture::prepared(root.path());
    assert_eq!(
        service
            .providers
            .iter()
            .map(super::java_constructor_provider::JavaConstructorProvider::name)
            .collect::<Vec<_>>(),
        ["preferences"]
    );
    let names: Vec<&str> = service
        .members
        .iter()
        .map(|member| member.name.as_str())
        .collect();
    assert_eq!(names, ["read", "write"]);
    assert!(
        service
            .source
            .contains("private final PreferenceStrings preferences;")
    );
    assert!(
        service
            .source
            .contains("ChoiceService(PreferenceStrings preferences)")
    );
    let (_, table) = JavaServiceTable::new(&values).file(&[service]);
    assert!(
        table.contains("new TacService0.ChoiceService(new BrowserPreferenceStrings(preferences))")
    );
    assert!(!table.contains("current"));
}

#[test]
fn an_existing_java_no_argument_factory_keeps_unrelated_constructor_overloads() {
    let root = tempfile::tempdir().expect("project");
    let source = "@Service\nclass ChoiceService {\n ChoiceService() {}\n ChoiceService(String unrelated) {}\n String answer() { return \"legacy\"; }\n}\n";
    let (service, values) = PreferenceFixture::prepared_source(root.path(), source);
    assert!(service.providers.is_empty());
    let (_, table) = JavaServiceTable::new(&values).file(&[service]);
    assert!(table.contains("new TacService0.ChoiceService()"));
    assert!(!table.contains("new BrowserPreferenceStrings"));
}

#[test]
#[ignore = "requires the installed JDK; validates generated Java activation ownership with a typed fake host"]
fn disposed_java_preference_providers_cannot_call_the_host_or_follow_new_activations() {
    let root = tempfile::tempdir().expect("project");
    let files = PreferenceFixture::staged(root.path());
    let classes = root.path().join("classes");
    PreferenceFixture::execute(
        Command::new("javac")
            .args(["--release", "21", "-nowarn", "-d"])
            .arg(&classes)
            .args(&files),
    );
    let report = PreferenceFixture::execute(
        Command::new("java")
            .arg("-ea")
            .arg("-cp")
            .arg(&classes)
            .arg("PreferenceMain"),
    );
    assert_eq!(report.trim(), "preference-ownership-qualified");
}

#[test]
fn private_final_publisher_and_preference_fields_have_ordered_browser_factories() {
    let root = tempfile::tempdir().expect("project");
    let source = "@Service\nclass ChoiceService {\n @Needs\n private final ApplicationEventPublisher events;\n @Needs\n private final PreferenceStrings preferences;\n String read() { return preferences.read(\"tachyon.platform\"); }\n}\n";
    let (service, values) = PreferenceFixture::prepared_source(root.path(), source);
    assert_eq!(
        service
            .providers
            .iter()
            .map(super::java_constructor_provider::JavaConstructorProvider::name)
            .collect::<Vec<_>>(),
        ["events", "preferences"]
    );
    assert!(
        service
            .source
            .contains("private final ApplicationEventPublisher events;")
    );
    assert!(service.source.contains("this.events = events;"));
    assert!(
        !service
            .source
            .contains("events = new ApplicationEventPublisher")
    );
    assert!(!service.members.iter().any(|member| member.name == "events"));
    let (_, table) = JavaServiceTable::new(&values).file(&[service]);
    assert!(table.contains("new TacService0.ChoiceService(new ApplicationEventPublisher(owner), new BrowserPreferenceStrings(preferences))"));
}

#[test]
#[ignore = "requires the installed JDK; validates captured Java publisher ownership and retained model envelopes"]
fn a_java_injected_publisher_cannot_publish_through_another_activation() {
    let root = tempfile::tempdir().expect("project");
    let model = root.path().join("client/models/PlatformChanged.java");
    std::fs::create_dir_all(model.parent().expect("model parent")).expect("models");
    std::fs::write(&model, "record PlatformChanged(String platform) {}\n").expect("model");
    let mut files = PreferenceFixture::staged(root.path());
    let host = root.path().join("TacHost.java");
    let source = PreferenceFixture::HOST.replace(
        "static void emit(String payload) {}",
        "static String last; static void emit(String payload) { last = payload; }",
    );
    std::fs::write(host, source).expect("typed fake JDK host");
    let driver = root.path().join("PublisherMain.java");
    std::fs::write(&driver, r#"import models.PlatformChanged;
final class PublisherMain {
    public static void main(String[] ignored) {
        TacActivation first = new TacActivation("/", "choice", "first");
        TacActivation second = new TacActivation("/", "choice", "second");
        ApplicationEventPublisher captured = new ApplicationEventPublisher(first);
        TacEvents.runFor(first);
        PlatformChanged changed = new PlatformChanged("macOS");
        require(captured.publishEvent(changed, true) == changed, "returns original typed model");
        TacEvents.drain();
        java.util.Map<?, ?> envelope = (java.util.Map<?, ?>) TacJsonReader.read(TacHost.last);
        require("first".equals(envelope.get("owner")), "captured owner envelope");
        require("PlatformChanged".equals(envelope.get("name")), "genuine model name");
        require(Boolean.TRUE.equals(envelope.get("retain")), "retention preserved");
        require("macOS".equals(((java.util.Map<?, ?>) envelope.get("value")).get("platform")), "typed payload");
        String before = TacHost.last;
        TacEvents.runFor(second);
        refused(() -> captured.publishEvent(changed, true));
        TacEvents.drain();
        require(before.equals(TacHost.last), "live old publisher cannot borrow new owner");
        first.dispose();
        TacEvents.runFor(second);
        refused(() -> captured.publishEvent(changed, true));
        ApplicationEventPublisher fresh = new ApplicationEventPublisher(second);
        fresh.publishEvent(new PlatformChanged("Windows"), true);
        TacEvents.drain();
        require(TacHost.last.contains("\"owner\":\"second\""), "new publisher active");
        TacEvents.runFor(second);
        new ApplicationEventPublisher().publishEvent(changed);
        TacEvents.drain();
        require(TacHost.last.contains("\"retain\":false"), "legacy noarg publisher compatible");
        second.dispose();
        System.out.println("publisher-ownership-qualified");
    }
    private static void refused(Runnable action) {
        try { action.run(); } catch (IllegalStateException expected) { return; }
        throw new AssertionError("inactive/disposed owner was accepted");
    }
    private static void require(boolean condition, String message) {
        if (!condition) throw new AssertionError(message);
    }
}
"#).expect("publisher driver");
    files.push(driver);
    let classes = root.path().join("classes");
    PreferenceFixture::execute(
        Command::new("javac")
            .args(["--release", "21", "-nowarn", "-d"])
            .arg(&classes)
            .args(files),
    );
    let output = PreferenceFixture::execute(
        Command::new("java")
            .arg("-ea")
            .arg("-cp")
            .arg(classes)
            .arg("PublisherMain"),
    );
    assert_eq!(output.trim(), "publisher-ownership-qualified");
}
