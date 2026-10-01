#![allow(clippy::expect_used)]

use super::ComposedTypes;
use std::path::Path;

struct Fixtures;

impl Fixtures {
    const CHOICE: &'static str = "@Service\nclass ChoiceService {\n    @Needs\n    private final PreferenceStrings preferences;\n    @Needs\n    private final ApplicationEventPublisher publisher;\n    final int count = 7;\n    private String secret = \"hidden\";\n    static String shared = \"hidden\";\n    void choose(String id) { preferences.write(\"platform\", id); }\n    @OnStart\n    void start() {}\n    @OnStop\n    void stop() {}\n}\n";

    fn write(root: &Path, path: &str, source: &str) {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("parent")).expect("folder");
        std::fs::write(file, source).expect("source");
    }

    fn generated(root: &Path) -> String {
        ComposedTypes::write(root).expect("types");
        std::fs::read_to_string(root.join(".tachyon/types/client/composed.d.ts"))
            .expect("types file")
    }
}

#[test]
fn java_browser_globals_use_the_validated_public_facade() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/services/ChoiceService.java",
        Fixtures::CHOICE,
    );
    let text = Fixtures::generated(root.path());
    assert!(text.contains("interface ChoiceService {"), "{text}");
    assert!(
        text.contains("\"choose\"(arg0: string | null): void;"),
        "{text}"
    );
    assert!(text.contains("readonly \"count\": number;"), "{text}");
    assert!(
        text.contains("new (properties?: {  }): ChoiceService"),
        "{text}"
    );
    for name in [
        "preferences",
        "publisher",
        "secret",
        "shared",
        "start",
        "stop",
    ] {
        assert!(!text.contains(name), "{name}: {text}");
    }
}

#[test]
fn java_component_properties_and_nested_dtos_retain_the_codec_types() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/models/Guide.java",
        "record Guide(String title, int pages, List<Map<String, Long>> history) {}\n",
    );
    Fixtures::write(
        root.path(),
        "client/models/Size.java",
        "enum Size { SHORT, LONG }\n",
    );
    Fixtures::write(
        root.path(),
        "client/services/TopicService.java",
        "@Service\n@NeedsAll\nclass TopicService {\n String title;\n int limit;\n @Needs\n private final PreferenceStrings preferences;\n List<Map<String, Guide>> entries(Integer index) { return null; }\n Size size() { return null; }\n Object opaque() { return null; }\n}\n",
    );
    let text = Fixtures::generated(root.path());
    for expected in [
        "new (properties: { \"title\": string | null; \"limit\": number; }): TopicService",
        "\"entries\"(arg0: number | null): Array<Record<string, __TachyonJavaModels.Guide | null> | null> | null;",
        "readonly \"history\": Array<Record<string, number | null> | null> | null;",
        "\"size\"(): __TachyonJavaModels.Size | null;",
        "type Size = \"SHORT\" | \"LONG\";",
        "\"opaque\"(): unknown;",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    assert!(!text.contains("preferences"), "{text}");
}

#[test]
fn unchanged_script_declarations_remain_byte_identical_and_other_java_roles_are_ignored() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/services/KeptService.ts",
        "@Service\nexport default class KeptService { answer(): number { return 42 } }\n",
    );
    let before = Fixtures::generated(root.path());
    Fixtures::write(
        root.path(),
        "server/services/ServerService.java",
        "@Service\nclass ServerService { int answer() { return 1; } }\n",
    );
    Fixtures::write(
        root.path(),
        "client/helpers/ToolsHelper.java",
        "@Helper\nclass ToolsHelper {}\n",
    );
    Fixtures::write(
        root.path(),
        "client/tests/ChoiceServiceTest.java",
        "@Test\nclass ChoiceServiceTest {}\n",
    );
    Fixtures::write(
        root.path(),
        "client/services/nested/tac.html",
        "<p>Entry</p>\n",
    );
    Fixtures::write(
        root.path(),
        "client/services/nested/HiddenService.java",
        Fixtures::CHOICE,
    );
    assert_eq!(Fixtures::generated(root.path()), before);
    assert_eq!(
        std::fs::read_to_string(root.path().join(".tachyon/types/server/composed.d.ts"))
            .expect("server"),
        "export {};\n"
    );
}

#[test]
fn generated_converter_dependencies_are_not_public_browser_constructor_arguments() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/models/Summary.java",
        "record Summary(String title) {}\n",
    );
    Fixtures::write(
        root.path(),
        "client/services/CatalogService.java",
        "import models.Summary;\n@Service\nclass CatalogService {\n @Needs\n ProductConverter converter;\n public Summary read() { return null; }\n}\n",
    );
    let text = Fixtures::generated(root.path());
    assert!(
        text.contains("new (properties?: {  }): CatalogService"),
        "{text}"
    );
    assert!(
        text.contains("\"read\"(): __TachyonJavaModels.Summary | null;"),
        "{text}"
    );
    assert!(!text.contains("ProductConverter"), "{text}");
}

#[test]
fn invalid_java_members_and_provider_constructors_keep_located_errors() {
    for body in [
        "char answer() { return 'a'; }",
        "BadService(String unsupported) {} int answer() { return 1; }",
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::write(
            root.path(),
            "client/services/BadService.java",
            &format!("@Service\nclass BadService {{ {body} }}\n"),
        );
        let failure = ComposedTypes::write(root.path()).expect_err("unsupported facade");
        let text = failure.to_string();
        assert!(
            text.contains("TY1405") && text.contains("client/services/BadService.java"),
            "{text}"
        );
        assert!(
            !root
                .path()
                .join(".tachyon/types/client/composed.d.ts")
                .exists()
        );
    }
}

#[test]
fn java_globals_refuse_script_and_nested_java_name_collisions() {
    for other in [
        "client/services/ChoiceService.ts",
        "client/services/nested/ChoiceService.java",
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::write(
            root.path(),
            "client/services/ChoiceService.java",
            Fixtures::CHOICE,
        );
        let source = if other.ends_with("ts") {
            "@Service\nexport default class ChoiceService {}\n"
        } else {
            Fixtures::CHOICE
        };
        Fixtures::write(root.path(), other, source);
        let failure = ComposedTypes::write(root.path()).expect_err("duplicate name");
        let text = failure.to_string();
        assert!(
            text.contains("TY1410") && text.contains("ChoiceService.java"),
            "{text}"
        );
    }
}

#[test]
fn java_model_refusals_retain_paths_and_duplicate_dtos_cannot_merge() {
    for (extra, source, wanted) in [
        (
            "client/models/Broken.java",
            "record Broken(char letter) {}\n",
            "client/models/Broken.java",
        ),
        (
            "client/models/nested/Guide.java",
            "record Guide(int pages) {}\n",
            "client/models/nested/Guide.java",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::write(
            root.path(),
            "client/services/ChoiceService.java",
            Fixtures::CHOICE,
        );
        Fixtures::write(
            root.path(),
            "client/models/Guide.java",
            "record Guide(String title) {}\n",
        );
        Fixtures::write(root.path(), extra, source);
        let failure = ComposedTypes::write(root.path()).expect_err("unsupported or duplicate DTO");
        let text = failure.to_string();
        assert!(text.contains("TY1405") && text.contains(wanted), "{text}");
        assert!(
            !root
                .path()
                .join(".tachyon/types/client/composed.d.ts")
                .exists()
        );
    }
}

#[cfg(unix)]
#[test]
fn java_metadata_never_follows_linked_source_files_or_folders() {
    let root = tempfile::tempdir().expect("project");
    let outside = tempfile::tempdir().expect("outside");
    Fixtures::write(outside.path(), "ChoiceService.java", Fixtures::CHOICE);
    std::fs::create_dir_all(root.path().join("client/services")).expect("services");
    std::os::unix::fs::symlink(
        outside.path().join("ChoiceService.java"),
        root.path().join("client/services/ChoiceService.java"),
    )
    .expect("file link");
    std::os::unix::fs::symlink(outside.path(), root.path().join("client/services/nested"))
        .expect("folder link");
    assert_eq!(Fixtures::generated(root.path()), "export {};\n");
}

#[test]
#[ignore = "requires an explicitly selected TypeScript compiler"]
fn java_composed_facades_typecheck_consumers_and_reject_wrong_arguments_and_private_providers() {
    let executable =
        std::env::var_os("TAC_TSC").expect("TAC_TSC must identify the installed tsc executable");
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/services/ChoiceService.java",
        Fixtures::CHOICE,
    );
    Fixtures::generated(root.path());
    Fixtures::write(
        root.path(),
        "consumer.ts",
        "const choice: ChoiceService = new ChoiceService();\nconst nothing: void = choice.choose('web');\nconst count: number = choice.count;\n// @ts-expect-error Java String arguments reject numbers.\nchoice.choose(42);\n// @ts-expect-error Injected providers stay inside the service.\nchoice.publisher = {};\n// @ts-expect-error Lifecycle hooks are not page methods.\nchoice.start();\n// @ts-expect-error Final public values are read-only.\nchoice.count = 9;\nvoid nothing; void count;\n",
    );
    let mut command = std::process::Command::new(executable);
    command.current_dir(root.path()).args([
        "--strict",
        "--noEmit",
        "--target",
        "ES2022",
        ".tachyon/types/client/composed.d.ts",
        "consumer.ts",
    ]);
    let output =
        crate::external_command::run_sync(&mut command, std::time::Duration::from_secs(30), 65_536)
            .expect("bounded tsc");
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
