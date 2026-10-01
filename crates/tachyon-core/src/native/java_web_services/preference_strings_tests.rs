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
        let path = root.join("client/services/ChoiceService.java");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, Self::SOURCE).expect("service");
        let models = JavaModels::read(root, Self::SOURCE).expect("models");
        let values = models.values();
        let authored = JavaService {
            instance: String::from("choice"),
            class: String::from("ChoiceService"),
            members: Vec::new(),
            required: Vec::new(),
            preferences: Vec::new(),
            path: path.strip_prefix(root).expect("relative").to_path_buf(),
            source: String::from(Self::SOURCE),
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
        let output = crate::external_command::run_sync(command, Duration::from_secs(60), 262_144)
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
    assert_eq!(service.preferences, ["preferences"]);
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
