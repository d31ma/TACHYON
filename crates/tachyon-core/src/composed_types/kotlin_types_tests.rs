#![allow(clippy::expect_used)]

use super::ComposedTypes;
use std::path::Path;

struct Fixtures;

impl Fixtures {
    const PANEL: &'static str = "import models.Guide\n\n@Service\n@NeedsAll\nclass PanelService {\n  val title: String\n  private val documentTitle: DocumentTitle\n  @Needs\n  private val publisher: ApplicationEventPublisher\n  var count: Int = 0\n  val fixed: String = \"ready\"\n  private val hidden = 1\n  fun increment() { count += 1 }\n  fun choose(id: String): Unit {}\n  fun optional(): String? = null\n  fun rows(): List<Guide?> = emptyList()\n  fun echo(guide: Guide): Guide = guide\n  fun opaque(value: Any?): Any? = value\n  @OnStart\n  fun start() {}\n  @OnStop\n  fun stop() {}\n}\n";
    const GUIDE: &'static str =
        "data class Guide(val title: String, val pages: Int, val labels: List<String?>)\n";

    fn write(root: &Path, path: &str, source: &str) {
        let file = root.join(path);
        std::fs::create_dir_all(file.parent().expect("parent")).expect("folder");
        std::fs::write(file, source).expect("source");
    }

    fn generated(root: &Path) -> String {
        ComposedTypes::write(root).expect("composed Kotlin types");
        std::fs::read_to_string(root.join(".tachyon/types/client/composed.d.ts"))
            .expect("types file")
    }

    fn panel(root: &Path) {
        Self::write(root, "client/models/Guide.kt", Self::GUIDE);
        Self::write(root, "client/services/PanelService.kt", Self::PANEL);
    }
}

#[test]
fn kotlin_facade_preserves_properties_unit_nullable_lists_and_owned_dtos() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::panel(root.path());
    let text = Fixtures::generated(root.path());
    for expected in [
        "interface PanelService {",
        "readonly \"title\": string;",
        "\"increment\"(): void;",
        "\"choose\"(arg0: string): void;",
        "\"optional\"(): string | null;",
        "\"rows\"(): Array<__TachyonKotlinModels.Guide | null>;",
        "\"echo\"(arg0: __TachyonKotlinModels.Guide): __TachyonKotlinModels.Guide;",
        "\"opaque\"(arg0: unknown | null): unknown | null;",
        "new (properties: { \"count\"?: number; \"title\": string; }): PanelService",
        "\"labels\": Array<string | null>;",
    ] {
        assert!(text.contains(expected), "{expected}: {text}");
    }
    for private in ["documentTitle", "publisher", "hidden", "start", "stop"] {
        assert!(!text.contains(private), "{private}: {text}");
    }
}

#[test]
fn unchanged_counter_block_method_gets_its_language_defined_unit_type() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/services/CounterService.kt",
        include_str!(
            "../../../../website/content/docs/first-app/add-a-service/client/services/CounterService.kt"
        ),
    );
    let text = Fixtures::generated(root.path());
    assert!(text.contains("\"count\": number;"), "{text}");
    assert!(text.contains("\"increment\"(): void;"), "{text}");
    assert!(
        text.contains("new (properties?: { \"count\"?: number; }): CounterService"),
        "{text}"
    );
}

#[test]
fn kotlin_composed_typing_refuses_inference_raw_maps_and_unknown_codecs_with_paths() {
    for body in [
        "fun answer() = 42",
        "val answer = 42",
        "fun entries(value: Map<String, Guide>): Map<String, Guide> = value",
        "fun entries(): Set<String> = emptySet()",
        "fun entries(): Array<String> = emptyArray()",
        "fun byte(): Byte = 1",
        "fun missing(): Unregistered = TODO()",
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::write(
            root.path(),
            "client/services/BadService.kt",
            &format!("@Service\nclass BadService {{\n  {body}\n}}\n"),
        );
        let failure =
            ComposedTypes::write(root.path()).expect_err("unsupported Kotlin composed member");
        let text = failure.to_string();
        assert!(
            text.contains("TY1405") && text.contains("client/services/BadService.kt"),
            "{text}"
        );
    }
}

#[test]
fn kotlin_authored_constructors_are_checked_before_internal_provider_lowering() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/services/GoodService.kt",
        "@Service\nclass GoodService(seed: Int = 0) {\n var count: Int = seed\n fun increment() {}\n}\n",
    );
    assert!(Fixtures::generated(root.path()).contains("interface GoodService"));
    Fixtures::write(
        root.path(),
        "client/services/BadService.kt",
        "@Service\nclass BadService(\n seed: Int\n) {\n var count: Int = seed\n}\n",
    );
    let text = ComposedTypes::write(root.path())
        .expect_err("unsupported authored arguments")
        .to_string();
    assert!(
        text.contains("no-argument constructor") && text.contains("client/services/BadService.kt"),
        "{text}"
    );
}

#[test]
fn kotlin_public_default_constructors_keep_legacy_overload_compatibility() {
    for declaration in [
        "class ChoiceService(seed: Int) {\n private val initial: Int = seed\n constructor() : this(0)\n fun answer(): Int = initial\n}\n",
        "class ChoiceService {\n constructor(seed: Int) {}\n constructor() {}\n fun answer(): Int = 42\n}\n",
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::write(
            root.path(),
            "client/services/ChoiceService.kt",
            &format!("@Service\n{declaration}"),
        );
        assert!(Fixtures::generated(root.path()).contains("interface ChoiceService"));
    }
    for declaration in [
        "class ChoiceService private constructor() {\n fun answer(): Int = 42\n}\n",
        "class ChoiceService {\n private constructor() {}\n fun answer(): Int = 42\n}\n",
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::write(
            root.path(),
            "client/services/ChoiceService.kt",
            &format!("@Service\n{declaration}"),
        );
        let text = ComposedTypes::write(root.path())
            .expect_err("unusable owned constructor")
            .to_string();
        assert!(
            text.contains("TY1405") && text.contains("client/services/ChoiceService.kt"),
            "{text}"
        );
    }
}

#[test]
fn kotlin_model_metadata_refusals_are_located_and_duplicate_names_cannot_merge() {
    for (path, source, wanted, diagnostic) in [
        (
            "client/models/Broken.kt",
            "data class Broken(val value: Map<String, Guide>)\n",
            "client/models/Broken.kt",
            "TY1310",
        ),
        (
            "client/models/Broken.kt",
            "data class Broken(val value: Short)\n",
            "client/models/Broken.kt",
            "TY1405",
        ),
        (
            "client/models/Broken.kt",
            "typealias Broken = String\n",
            "client/models/Broken.kt",
            "TY1405",
        ),
        (
            "client/models/nested/Guide.kt",
            "data class Guide(val pages: Int)\n",
            "client/models/nested/Guide.kt",
            "TY1405",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::panel(root.path());
        Fixtures::write(root.path(), path, source);
        let text = ComposedTypes::write(root.path())
            .expect_err("unsupported or repeated DTO")
            .to_string();
        assert!(text.contains(diagnostic) && text.contains(wanted), "{text}");
    }
}

#[test]
fn kotlin_service_globals_refuse_script_java_and_nested_kotlin_collisions() {
    for (path, source) in [
        (
            "client/services/ChoiceService.ts",
            "@Service\nexport default class ChoiceService {}\n",
        ),
        (
            "client/services/ChoiceService.java",
            "@Service\nclass ChoiceService { int answer() { return 1; } }\n",
        ),
        (
            "client/services/nested/ChoiceService.kt",
            "@Service\nclass ChoiceService {\n fun answer(): Int = 1\n}\n",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        Fixtures::write(
            root.path(),
            "client/services/ChoiceService.kt",
            "@Service\nclass ChoiceService {\n fun answer(): Int = 42\n}\n",
        );
        Fixtures::write(root.path(), path, source);
        let text = ComposedTypes::write(root.path())
            .expect_err("composed collision")
            .to_string();
        assert!(
            text.contains("TY1410") && text.contains("ChoiceService.kt"),
            "{text}"
        );
    }
}

#[test]
fn kotlin_other_realms_roles_and_entry_folders_leave_existing_script_java_output_identical() {
    let root = tempfile::tempdir().expect("project");
    Fixtures::write(
        root.path(),
        "client/services/JavaService.java",
        "@Service\nclass JavaService { void choose(String id) {} }\n",
    );
    Fixtures::write(
        root.path(),
        "client/services/ScriptService.ts",
        "@Service\nexport default class ScriptService { answer(): number { return 42 } }\n",
    );
    let before = Fixtures::generated(root.path());
    for (path, source) in [
        (
            "server/services/OtherService.kt",
            "@Service\nclass OtherService {}\n",
        ),
        (
            "client/tests/OtherServiceTest.kt",
            "@Test\nclass OtherServiceTest {}\n",
        ),
        (
            "client/helpers/OtherHelper.kt",
            "@Helper\nclass OtherHelper {}\n",
        ),
        ("client/services/nested/tac.html", "<p>Entry</p>\n"),
        (
            "client/services/nested/HiddenService.kt",
            "@Service\nclass HiddenService {\n fun answer(): Int = 42\n}\n",
        ),
    ] {
        Fixtures::write(root.path(), path, source);
    }
    assert_eq!(Fixtures::generated(root.path()), before);
}

#[test]
#[ignore = "requires an explicitly selected TypeScript compiler"]
fn kotlin_composed_facades_typecheck_real_consumer_contracts() {
    let executable = std::env::var_os("TAC_TSC").expect("TAC_TSC identifies installed tsc");
    let root = tempfile::tempdir().expect("project");
    Fixtures::panel(root.path());
    Fixtures::generated(root.path());
    Fixtures::write(
        root.path(),
        "consumer.ts",
        "const panel = new PanelService({title: 'Ready'});\nconst result: void = panel.choose('web');\nconst value: string | null = panel.optional();\nconst guide = panel.echo({title: 'Guide', pages: 1, labels: ['ready', null]});\nconst pages: number = guide.pages;\n// @ts-expect-error Kotlin String rejects numeric arguments.\npanel.choose(42);\n// @ts-expect-error Injected providers are not public facade members.\npanel.documentTitle.set('hidden');\n// @ts-expect-error Lifecycle is owned by activation.\npanel.start();\n// @ts-expect-error Kotlin val properties cannot be written.\npanel.title = 'new';\n// @ts-expect-error NeedsAll component properties are required.\nnew PanelService();\nvoid result; void value; void pages;\n",
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
