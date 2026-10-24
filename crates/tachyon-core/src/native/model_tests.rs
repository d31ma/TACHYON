#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};
use std::process::Command;

struct Sources;

impl Sources {
    const RUST_MODEL: &str = "#[derive(Clone, Debug)]\npub struct Guide {\n    pub title: String,\n    pub pages: i64,\n}\n";
    const RUST_SERVICE: &str = r#"use crate::models::Guide;
use crate::models::Guide as ReadingGuide;
use crate::models as catalog;

#[Service()]
#[derive(Default)]
struct GuideService {
    pages: i64,
}

impl GuideService {
    fn read(&mut self) -> String {
        let guide: crate::models::Guide = Guide { title: String::from("Guide"), pages: 3 };
        let alias: ReadingGuide = guide.clone();
        let catalogued: catalog::Guide = alias;
        let _ = crate::TacValue::Null;
        self.pages = catalogued.pages;
        format!("{} {}", catalogued.title, catalogued.pages)
    }
}
"#;
    const SWIFT_MODEL: &str = "import Foundation\n\nstruct Guide: Codable, Equatable {\n    let title: String\n    var pages: Int = 0\n}\n";
    const SWIFT_KIND_MODEL: &str = "enum Kind: String, Codable {\n    case short, long\n}\n";
    const SWIFT_SERVICE: &str = "@Service\nclass GuideService {\n    var pages: Int = 0\n    func read() -> String {\n        let guide = Guide(title: \"Guide\", pages: 3)\n        pages = guide.pages\n        return guide.title + \" \" + Kind.long.rawValue\n    }\n}\n";
    const SWIFT_WAITING_SERVICE: &str = "@Service\nclass GuideService {\n    func load(_ name: String) async throws -> String {\n        try await Task.sleep(nanoseconds: 1_000_000)\n        return await MainActor.run { name + \" loaded\" }\n    }\n}\n";
    const SWIFT_CALLER: &str = "@main\nstruct CompanionCaller {\n    static let request = \"{\\\"route\\\":\\\"/\\\",\\\"owner\\\":\\\"6F9619FF-8B86-D011-B42D-00C04FC964FF\\\",\\\"service\\\":\\\"guide\\\",\"\n\n    static func main() {\n        print(tacNativeInvoke(request + \"\\\"op\\\":\\\"init\\\"}\"))\n        print(tacNativeInvoke(request + \"\\\"op\\\":\\\"call\\\",\\\"name\\\":\\\"load\\\",\\\"args\\\":[\\\"Guide\\\"]}\"))\n    }\n}\n";
    const KOTLIN_MODEL: &str = "data class Guide(val title: String, val pages: Int)\n";
    const KOTLIN_KIND_MODEL: &str = "enum class Kind { SHORT, LONG }\n";
    const KOTLIN_SERVICE: &str = "import models.Guide\nimport models.Kind\n\n@Service\nclass GuideService {\n    var pages: Int = 0\n    fun read(): String {\n        val guide = Guide(\"Guide\", 3)\n        pages = guide.pages\n        return guide.title + \" \" + Kind.LONG.name\n    }\n}\n";
}

fn available(program: &str, flag: &str) -> bool {
    Command::new(program)
        .arg(flag)
        .output()
        .is_ok_and(|output| output.status.success())
}

fn write(root: &Path, relative: &str, source: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(&path, source).expect("source");
    path
}

fn companion(
    root: &Path,
    language: NativeCompanion,
    extension: &str,
    model: &str,
    service: &str,
) -> NativeCompanionInput {
    write(root, &format!("client/models/Guide.{extension}"), model);
    let source = write(
        root,
        &format!("client/services/GuideService.{extension}"),
        service,
    );
    NativeCompanionInput {
        snapshot_root: root.to_path_buf(),
        language,
        source,
        service: String::from("guide"),
        class: String::from("GuideService"),
        web_assembly: false,
        application_provider: false,
    }
}

fn assert_success(command: &mut Command) {
    let output = command.output().expect("compiler");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::test]
async fn a_rust_native_companion_compiles_with_its_models() {
    if !available("rustc", "--version") {
        eprintln!("skipping: rustc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let input = companion(
        root.path(),
        NativeCompanion::Rust,
        "rs",
        Sources::RUST_MODEL,
        Sources::RUST_SERVICE,
    );
    let staged = super::rust::stage(&[input], root.path(), "app")
        .expect("stage")
        .expect("a Rust companion");
    let program = std::fs::read_to_string(&staged).expect("staged");
    assert!(program.contains("pub mod models"), "{program}");
    let library = root.path().join("companion.a");
    super::rust::compile(&staged, super::rust::Linkage::Static, None, &library)
        .await
        .expect("the companion compiles with its models");
}

#[test]
fn a_swift_native_companion_compiles_with_its_models() {
    if !cfg!(target_os = "macos") || !available("swiftc", "--version") {
        eprintln!("skipping: swiftc on macOS is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let input = companion(
        root.path(),
        NativeCompanion::Swift,
        "swift",
        Sources::SWIFT_MODEL,
        Sources::SWIFT_SERVICE,
    );
    write(
        root.path(),
        "client/models/Kind.swift",
        Sources::SWIFT_KIND_MODEL,
    );
    let staged = super::macos::stage_swift_companion(&[input], root.path(), None)
        .expect("stage")
        .expect("a Swift companion");
    let json = root.path().join("apple_json.swift");
    std::fs::write(&json, super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST).expect("json");
    assert_success(
        Command::new("swiftc")
            .args(["-parse-as-library", "-typecheck"])
            .arg(&staged)
            .arg(&json),
    );
}

#[test]
fn a_swift_native_companion_awaits_an_async_method_on_the_owner_thread() {
    if !cfg!(target_os = "macos") || !available("swiftc", "--version") {
        eprintln!("skipping: swiftc on macOS is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let input = companion(
        root.path(),
        NativeCompanion::Swift,
        "swift",
        Sources::SWIFT_MODEL,
        Sources::SWIFT_WAITING_SERVICE,
    );
    let staged = super::macos::stage_swift_companion(&[input], root.path(), None)
        .expect("stage")
        .expect("a Swift companion");
    let json = write(
        root.path(),
        "apple_json.swift",
        super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST,
    );
    let caller = write(root.path(), "caller.swift", Sources::SWIFT_CALLER);
    let program = root.path().join("caller");
    assert_success(
        Command::new("swiftc")
            .arg("-parse-as-library")
            .arg(&staged)
            .arg(&json)
            .arg(&caller)
            .arg("-o")
            .arg(&program),
    );
    let output = Command::new(&program).output().expect("the caller runs");
    let printed = String::from_utf8_lossy(&output.stdout);
    assert!(
        printed.contains("{\"value\":\"Guide loaded\"}"),
        "{printed}"
    );
}

#[test]
fn a_kotlin_native_companion_compiles_with_its_models() {
    let android = std::env::var_os("ANDROID_HOME").map(PathBuf::from);
    let platform = android.map(|home| home.join("platforms/android-35/android.jar"));
    let Some(platform) = platform.filter(|path| path.is_file()) else {
        eprintln!("skipping: the Android platform jar is unavailable");
        return;
    };
    if !available("kotlinc", "-version") {
        eprintln!("skipping: kotlinc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let input = companion(
        root.path(),
        NativeCompanion::Kotlin,
        "kt",
        Sources::KOTLIN_MODEL,
        Sources::KOTLIN_SERVICE,
    );
    write(
        root.path(),
        "client/models/Kind.kt",
        Sources::KOTLIN_KIND_MODEL,
    );
    let project = root.path().join("android");
    let staged = super::android::stage_companion(&[input], &project, "app.guide", "app/guide")
        .expect("stage");
    assert!(staged);
    let sources = project.join("app/src/main/kotlin");
    assert!(sources.join("models/Guide.kt").is_file());
    assert_success(
        Command::new("kotlinc")
            .arg("-classpath")
            .arg(&platform)
            .arg(&sources)
            .arg("-d")
            .arg(root.path().join("classes")),
    );
}

fn required_companions(root: &Path, language: NativeCompanion) -> Vec<NativeCompanionInput> {
    let extension = if language == NativeCompanion::Swift {
        "swift"
    } else {
        "kt"
    };
    let source = write(
        root,
        &format!("client/services/EveryService.{extension}"),
        "@Service\n@NeedsAll\nclass EveryService {\n    var count: Int = 0\n    var label: String = \"kept\"\n}\n",
    );
    vec![NativeCompanionInput {
        snapshot_root: root.to_path_buf(),
        language,
        source,
        service: String::from("every"),
        class: String::from("EveryService"),
        web_assembly: false,
        application_provider: false,
    }]
}

#[test]
fn swift_and_kotlin_registries_list_the_required_properties() {
    for (language, every) in [
        (
            NativeCompanion::Swift,
            "static let tacRequired: [String] = [\"*\"]",
        ),
        (
            NativeCompanion::Kotlin,
            "val tacRequired: List<String> = listOf(\"*\")",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        let inputs = required_companions(root.path(), language);
        let program = super::registry::source(&inputs, language)
            .expect("registry")
            .expect("services");
        assert!(program.contains(every), "{program}");
        assert!(program.contains("tacServiceRequired"), "{program}");
    }
}

#[test]
fn a_swift_companion_receives_the_required_properties_the_page_sends() {
    if !cfg!(target_os = "macos") || !available("swiftc", "--version") {
        eprintln!("skipping: swiftc on macOS is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let inputs = required_companions(root.path(), NativeCompanion::Swift);
    let staged = super::macos::stage_swift_companion(&inputs, root.path(), None)
        .expect("stage")
        .expect("a Swift companion");
    let json = write(
        root.path(),
        "apple_json.swift",
        super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST,
    );
    let main = write(
        root.path(),
        "main.swift",
        "func ask(_ service: String, _ owner: String, _ body: String) -> String {\n    tacNativeInvoke(\"{\\\"route\\\":\\\"/\\\",\\\"owner\\\":\\\"\" + owner + \"\\\",\\\"service\\\":\\\"\" + service + \"\\\",\" + body + \"}\")\n}\nfor (service, owner) in [(\"every\", \"00000000-0000-4000-8000-000000000002\")] {\n    _ = ask(service, owner, \"\\\"op\\\":\\\"init\\\",\\\"props\\\":{\\\"count\\\":5,\\\"label\\\":\\\"given\\\"}\")\n    print(service, ask(service, owner, \"\\\"op\\\":\\\"get\\\",\\\"name\\\":\\\"count\\\"\"), ask(service, owner, \"\\\"op\\\":\\\"get\\\",\\\"name\\\":\\\"label\\\"\"))\n}\n",
    );
    let program = root.path().join("companion");
    assert_success(
        Command::new("swiftc")
            .arg(&staged)
            .arg(&json)
            .arg(&main)
            .arg("-o")
            .arg(&program),
    );
    let output = Command::new(&program).output().expect("companion runs");
    let printed = String::from_utf8_lossy(&output.stdout);
    assert_eq!(printed, "every {\"value\":5} {\"value\":\"given\"}\n");
}

struct ConstructedSources;

impl ConstructedSources {
    const SWIFT_SERVICE: &str = "@Service\n@NeedsAll\nclass BadgeService {\n    let path: String\n    var label: String\n    var count: Int = 0\n    func title() -> String { label + path }\n}\n";
    const KOTLIN_SERVICE: &str = "@Service\n@NeedsAll\nclass BadgeService {\n    val path: String\n    val size: Int\n    lateinit var label: String\n    var count: Int = 0\n    fun title(): String = label + path\n}\n";
    const ANDROID_STUB: &str = "package android.content\n\ninterface SharedPreferences {\n    val all: Map<String, *>\n    fun edit(): Editor\n    interface Editor {\n        fun putInt(key: String, value: Int): Editor\n        fun putLong(key: String, value: Long): Editor\n        fun putBoolean(key: String, value: Boolean): Editor\n        fun putFloat(key: String, value: Float): Editor\n        fun putString(key: String, value: String?): Editor\n        fun remove(key: String): Editor\n        fun apply()\n    }\n}\n";
    const JSON_STUB: &str = "package org.json\n\nclass JSONTokener(text: String) {\n    fun nextValue(): Any? = TODO()\n}\n\nclass JSONArray {\n    fun length(): Int = TODO()\n    fun opt(index: Int): Any? = TODO()\n}\n\nclass JSONObject(text: String) {\n    fun keys(): Iterator<String> = TODO()\n    fun opt(name: String): Any? = TODO()\n    fun optString(name: String): String = TODO()\n    fun optJSONObject(name: String): JSONObject? = TODO()\n    fun optJSONArray(name: String): JSONArray? = TODO()\n    companion object {\n        @JvmField val NULL: Any = Any()\n        fun quote(value: String?): String = TODO()\n    }\n}\n";
}

fn constructed_companion(root: &Path, language: NativeCompanion) -> Vec<NativeCompanionInput> {
    let (extension, service) = if language == NativeCompanion::Swift {
        ("swift", ConstructedSources::SWIFT_SERVICE)
    } else {
        ("kt", ConstructedSources::KOTLIN_SERVICE)
    };
    let source = write(
        root,
        &format!("client/services/BadgeService.{extension}"),
        service,
    );
    vec![NativeCompanionInput {
        snapshot_root: root.to_path_buf(),
        language,
        source,
        service: String::from("badge"),
        class: String::from("BadgeService"),
        web_assembly: false,
        application_provider: false,
    }]
}

#[test]
fn swift_and_kotlin_generate_the_constructor_for_required_immutable_fields() {
    for (language, constructor, created) in [
        (
            NativeCompanion::Swift,
            "init?(tacProps: [String: Any]) {\nguard let path = tachyonSwiftJSONValue(tacProps[\"path\"]) as? String else { return nil }\nself.path = path\nguard let label = tachyonSwiftJSONValue(tacProps[\"label\"]) as? String else { return nil }\nself.label = label\n}\n",
            "guard let tacInstance = BadgeService(tacProps: tacProps) else { return nil }",
        ),
        (
            NativeCompanion::Kotlin,
            "internal constructor(tacProps: Map<String, Any?>?, tacPage: TacPageScope) {\nthis.path = tacRequiredValue(tacProps, \"path\").let { tacString(it) }\nthis.size = tacRequiredValue(tacProps, \"size\").let { tacInt(it) }\nthis.label = tacRequiredValue(tacProps, \"label\").let { tacString(it) }\n}\n",
            "val tacInstance = BadgeService(tacProps, tacPage)",
        ),
    ] {
        let root = tempfile::tempdir().expect("project");
        let inputs = constructed_companion(root.path(), language);
        let program = super::registry::source(&inputs, language)
            .expect("registry")
            .expect("services");
        assert!(program.contains(constructor), "{program}");
        assert!(program.contains(created), "{program}");
    }
}

#[test]
fn a_swift_companion_builds_its_let_fields_from_the_page_properties() {
    if !cfg!(target_os = "macos") || !available("swiftc", "--version") {
        eprintln!("skipping: swiftc on macOS is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let inputs = constructed_companion(root.path(), NativeCompanion::Swift);
    let staged = super::macos::stage_swift_companion(&inputs, root.path(), None)
        .expect("stage")
        .expect("a Swift companion");
    let json = write(
        root.path(),
        "apple_json.swift",
        super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST,
    );
    let main = write(
        root.path(),
        "main.swift",
        "func ask(_ owner: String, _ body: String) -> String {\n    tacNativeInvoke(\"{\\\"route\\\":\\\"/\\\",\\\"owner\\\":\\\"\" + owner + \"\\\",\\\"service\\\":\\\"badge\\\",\" + body + \"}\")\n}\nlet given = \"00000000-0000-4000-8000-000000000003\"\n_ = ask(given, \"\\\"op\\\":\\\"init\\\",\\\"props\\\":{\\\"path\\\":\\\"/docs\\\",\\\"label\\\":\\\"Docs \\\"}\")\nprint(ask(given, \"\\\"op\\\":\\\"call\\\",\\\"name\\\":\\\"title\\\",\\\"args\\\":[]\"))\nprint(ask(\"00000000-0000-4000-8000-000000000004\", \"\\\"op\\\":\\\"init\\\",\\\"props\\\":{\\\"label\\\":\\\"Docs\\\"}\"))\n",
    );
    let program = root.path().join("companion");
    assert_success(
        Command::new("swiftc")
            .arg(&staged)
            .arg(&json)
            .arg(&main)
            .arg("-o")
            .arg(&program),
    );
    let output = Command::new(&program).output().expect("companion runs");
    let printed = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        printed,
        "{\"value\":\"Docs /docs\"}\n{\"error\":\"Native service is missing a required property or it has the wrong type.\",\"code\":\"TY_NATIVE_REQUIRED\"}\n"
    );
}

#[test]
fn a_kotlin_companion_with_required_val_fields_compiles() {
    if !available("kotlinc", "-version") {
        eprintln!("skipping: kotlinc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let inputs = constructed_companion(root.path(), NativeCompanion::Kotlin);
    let project = root.path().join("android");
    let staged = super::android::stage_companion(&inputs, &project, "app.badge", "app/badge")
        .expect("stage");
    assert!(staged);
    let stubs = root.path().join("stubs");
    write(
        &stubs,
        "android/content/SharedPreferences.kt",
        ConstructedSources::ANDROID_STUB,
    );
    write(
        &stubs,
        "org/json/JSONObject.kt",
        ConstructedSources::JSON_STUB,
    );
    assert_success(
        Command::new("kotlinc")
            .arg(project.join("app/src/main/kotlin"))
            .arg(&stubs)
            .arg("-d")
            .arg(root.path().join("classes")),
    );
}
