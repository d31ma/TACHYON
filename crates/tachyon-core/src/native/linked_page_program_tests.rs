#![allow(clippy::expect_used)]

use super::managed_capability_tests::ManagedCapabilityCheck;
use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use std::path::{Path, PathBuf};
use std::process::Command;

struct LinkedPageCheck;

impl LinkedPageCheck {
    const SWIFT_FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/ProgramState.swift",
            "struct ProgramState: Codable {\n    let identity: String\n    let count: Int\n}\n",
        ),
        (
            "client/remotes/PageProgramClient.swift",
            "@HTTPClient({\"adapter\":\"pageprogram\",\"path\":\"/program\",\"setup\":\"PageProgramSetup\"})\nprotocol PageProgramClient {\n    @GetRequest(\"/read\")\n    func read() async throws -> ProgramState\n\n    @PostRequest(\"/increment\")\n    func increment() async throws -> ProgramState\n}\n",
        ),
        (
            "client/setups/PageProgramSetup.swift",
            "@Setup\nfinal class PageProgramSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        let identity = String(UInt64.random(in: 1...UInt64.max), radix: 16)\n        var count = 0\n        var adapters = BehaviorAdapters()\n        adapters.http[\"pageprogram\"] = BehaviorHTTPAdapter { request, _ in\n            if try request.get(\"method\").text == \"POST\", try request.get(\"path\").text.hasSuffix(\"/increment\") {\n                count += 1\n            }\n            return .object([\"identity\": .string(identity), \"count\": .number(Double(count))])\n        }\n        return adapters\n    }\n}\n",
        ),
    ];
    const SWIFT_SERVICE: &'static str = "@Service\nfinal class __CLASS__ {\n    @Needs\n    private let pageProgramClient: PageProgramClient\n    var identity: String = \"\"\n    var count: Int = 0\n\n    func inspect() async throws {\n        show(try await pageProgramClient.read())\n    }\n\n    func increment() async throws {\n        show(try await pageProgramClient.increment())\n    }\n\n    private func show(_ state: ProgramState) {\n        identity = state.identity\n        count = state.count\n    }\n}\n";
    const SWIFT_CALLER: &'static str = "@main\nstruct CompanionCaller {\n    static func ask(_ service: String, _ rest: String) -> String {\n        tacNativeInvoke(\"{\\\"route\\\":\\\"/\\\",\\\"owner\\\":\\\"6F9619FF-8B86-D011-B42D-00C04FC964\" + (service == \"writer\" ? \"F1\" : \"F2\") + \"\\\",\\\"service\\\":\\\"\" + service + \"\\\",\" + rest + \"}\")\n    }\n\n    static func main() {\n        for service in [\"writer\", \"reader\"] {\n            print(service + \" init \" + ask(service, \"\\\"op\\\":\\\"init\\\"\"))\n        }\n        print(\"writer increment \" + ask(\"writer\", \"\\\"op\\\":\\\"call\\\",\\\"name\\\":\\\"increment\\\",\\\"args\\\":[]\"))\n        print(\"reader inspect \" + ask(\"reader\", \"\\\"op\\\":\\\"call\\\",\\\"name\\\":\\\"inspect\\\",\\\"args\\\":[]\"))\n        for service in [\"writer\", \"reader\"] {\n            for field in [\"identity\", \"count\"] {\n                print(service + \" \" + field + \" \" + ask(service, \"\\\"op\\\":\\\"get\\\",\\\"name\\\":\\\"\" + field + \"\\\"\"))\n            }\n        }\n    }\n}\n";
    const KOTLIN_FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/ProgramState.kt",
            "data class ProgramState(val identity: String, val count: Int)\n",
        ),
        (
            "client/remotes/PageProgramClient.kt",
            "import models.ProgramState\n\n@HTTPClient({\"adapter\":\"pageprogram\",\"path\":\"/program\",\"setup\":\"PageProgramSetup\"})\ninterface PageProgramClient {\n    @GetRequest(\"/read\")\n    suspend fun read(): ProgramState\n\n    @PostRequest(\"/increment\")\n    suspend fun increment(): ProgramState\n}\n",
        ),
        (
            "client/setups/PageProgramSetup.kt",
            "@Setup\nclass PageProgramSetup {\n    companion object {\n        fun configure(execution: Behaviors.Execution): Behaviors.Configuration {\n            val identity = kotlin.random.Random.nextLong().toULong().toString(16)\n            var count = 0\n            val program = Behaviors.HttpAdapter { request, _ ->\n                val path = request[\"path\"] as? String ?: \"\"\n                if (request[\"method\"] == \"POST\" && path.endsWith(\"/increment\")) count += 1\n                mapOf(\"identity\" to identity, \"count\" to count)\n            }\n            return Behaviors.Configuration(http = mapOf(\"pageprogram\" to program))\n        }\n    }\n}\n",
        ),
    ];
    const KOTLIN_SERVICE: &'static str = "import models.ProgramState\n\n@Service\nclass __CLASS__ {\n    @Needs\n    private val pageProgramClient: PageProgramClient\n    var identity: String = \"\"\n    var count: Int = 0\n\n    suspend fun inspect() {\n        show(pageProgramClient.read())\n    }\n\n    suspend fun increment() {\n        show(pageProgramClient.increment())\n    }\n\n    private fun show(state: ProgramState) {\n        identity = state.identity\n        count = state.count\n    }\n}\n";
    const KOTLIN_CALLER: &'static str = r#"
fun ask(service: String, rest: String): String =
    tacInvokeOwned("{\"route\":\"/\",\"service\":\"$service\",\"owner\":\"00000000-0000-4000-8000-00000000000${if (service == "writer") 1 else 2}\",$rest}")

fun main() {
    for (service in listOf("writer", "reader")) println("$service init ${ask(service, "\"op\":\"init\"")}")
    println("writer increment ${ask("writer", "\"op\":\"call\",\"name\":\"increment\",\"args\":[]")}")
    println("reader inspect ${ask("reader", "\"op\":\"call\",\"name\":\"inspect\",\"args\":[]")}")
    for (service in listOf("writer", "reader")) {
        for (field in listOf("identity", "count")) {
            println("$service $field ${ask(service, "\"op\":\"get\",\"name\":\"$field\"")}")
        }
    }
}
"#;
    const CSHARP_FILES: [(&'static str, &'static str); 3] = [
        (
            "client/models/ProgramState.cs",
            include_str!("../../tests/fixtures/native-csharp-page-program/ProgramState.cs"),
        ),
        (
            "client/remotes/PageProgramClient.cs",
            include_str!("../../tests/fixtures/native-csharp-page-program/PageProgramClient.cs"),
        ),
        (
            "client/setups/PageProgramSetup.cs",
            include_str!("../../tests/fixtures/native-csharp-page-program/PageProgramSetup.cs"),
        ),
    ];
    const CSHARP_SERVICE: &'static str =
        include_str!("../../tests/fixtures/native-csharp-page-program/ProgramService.cs");
    const CSHARP_DRIVER: &'static str =
        include_str!("../../tests/fixtures/native-csharp-page-program/PageProgramDriver.cs");
    const CSHARP_PROJECT: &'static str = "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><TargetFramework>net9.0</TargetFramework><OutputType>Exe</OutputType><Nullable>disable</Nullable><AllowUnsafeBlocks>true</AllowUnsafeBlocks><InvariantGlobalization>true</InvariantGlobalization><EnableDefaultCompileItems>false</EnableDefaultCompileItems></PropertyGroup><ItemGroup><Compile Include=\"TachyonCompanion.cs\" /><Compile Include=\"PageProgramDriver.cs\" /></ItemGroup></Project>";
    const SERVICES: [(&'static str, &'static str); 2] =
        [("writer", "WriterService"), ("reader", "ReaderService")];

    fn write(root: &Path, relative: &str, source: &str) -> PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, source).expect("source");
        path
    }

    fn inputs(
        root: &Path,
        language: NativeCompanion,
        (files, service): (&[(&str, &str)], &str),
    ) -> Vec<NativeCompanionInput> {
        for (relative, source) in files {
            Self::write(root, relative, source);
        }
        let extension = language.model_extension();
        Self::SERVICES
            .iter()
            .map(|(instance, class)| NativeCompanionInput {
                snapshot_root: root.to_path_buf(),
                language,
                source: Self::write(
                    root,
                    &format!("client/services/{class}.{extension}"),
                    &service.replace("__CLASS__", class),
                ),
                service: (*instance).to_owned(),
                class: (*class).to_owned(),
                web_assembly: false,
                application_provider: false,
            })
            .collect()
    }

    fn swift_inputs(root: &Path) -> Vec<NativeCompanionInput> {
        Self::inputs(
            root,
            NativeCompanion::Swift,
            (&Self::SWIFT_FILES, Self::SWIFT_SERVICE),
        )
    }

    fn kotlin_inputs(root: &Path) -> Vec<NativeCompanionInput> {
        Self::inputs(
            root,
            NativeCompanion::Kotlin,
            (&Self::KOTLIN_FILES, Self::KOTLIN_SERVICE),
        )
    }

    fn csharp_inputs(root: &Path) -> Vec<NativeCompanionInput> {
        Self::inputs(
            root,
            NativeCompanion::CSharp,
            (&Self::CSHARP_FILES, Self::CSHARP_SERVICE),
        )
    }

    fn csharp_run(root: &Path) -> String {
        let inputs = Self::csharp_inputs(root);
        let project = super::windows::WindowsHostGenerator::staged_companion(
            &inputs,
            root,
            ("dev.tachyon.page-program", None),
        )
        .expect("stage")
        .expect("a C# companion");
        Self::write(&project, "TachyonCompanion.csproj", Self::CSHARP_PROJECT);
        Self::write(&project, "PageProgramDriver.cs", Self::CSHARP_DRIVER);
        let mut command = Command::new("dotnet");
        command.arg("run").arg("--project").arg(&project);
        let output = crate::external_command::run_sync(
            &mut command,
            std::time::Duration::from_mins(4),
            65_536,
        )
        .expect("supervised .NET page program");
        let printed = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.status.success(), "{printed}");
        printed
    }

    fn available(tool: &str, argument: &str) -> bool {
        Command::new(tool)
            .arg(argument)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn answer(output: &str, prefix: &str) -> String {
        output
            .lines()
            .find_map(|line| line.strip_prefix(prefix))
            .unwrap_or_default()
            .to_owned()
    }

    fn assert_once(source: &str, declarations: &[&str]) {
        for declaration in declarations {
            assert_eq!(
                source.matches(declaration).count(),
                1,
                "{declaration}:\n{source}"
            );
        }
    }

    fn assert_shared(output: &str) {
        let writer = Self::answer(output, "writer identity ");
        assert!(writer.starts_with("{\"value\":\""), "{output}");
        assert_eq!(writer, Self::answer(output, "reader identity "), "{output}");
        assert_eq!(
            Self::answer(output, "reader count "),
            "{\"value\":1}",
            "{output}"
        );
    }
}

#[test]
fn two_linked_swift_services_keep_one_copy_of_their_setup_and_client() {
    let root = tempfile::tempdir().expect("project");
    let inputs = LinkedPageCheck::swift_inputs(root.path());
    let source = super::registry::source(&inputs, NativeCompanion::Swift)
        .expect("the linked services lower")
        .expect("a Swift program");
    LinkedPageCheck::assert_once(
        &source,
        &[
            "final class PageProgramSetup",
            "final class WriterService",
            "final class ReaderService",
            "private enum TachyonService0 {",
            "private enum TachyonService1 {",
        ],
    );
}

#[test]
fn two_linked_swift_services_share_one_setup_configuration() {
    if !LinkedPageCheck::available("swiftc", "--version") {
        eprintln!("skipping: swiftc is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let inputs = LinkedPageCheck::swift_inputs(root.path());
    let staged = super::macos::stage_swift_companion(&inputs, root.path(), None)
        .expect("stage")
        .expect("a Swift companion");
    let json = LinkedPageCheck::write(
        root.path(),
        "apple_json.swift",
        super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST,
    );
    let caller = LinkedPageCheck::write(root.path(), "caller.swift", LinkedPageCheck::SWIFT_CALLER);
    let program = root.path().join("caller");
    let compiled = Command::new("swiftc")
        .arg("-parse-as-library")
        .args([&staged, &json, &caller])
        .arg("-o")
        .arg(&program)
        .output()
        .expect("swiftc");
    assert!(
        compiled.status.success(),
        "{}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let output = Command::new(&program).output().expect("the caller runs");
    let printed = String::from_utf8_lossy(&output.stdout);
    println!("{printed}");
    LinkedPageCheck::assert_shared(&printed);
}

#[test]
fn two_linked_kotlin_services_keep_one_copy_of_their_setup_and_client() {
    let root = tempfile::tempdir().expect("project");
    let inputs = LinkedPageCheck::kotlin_inputs(root.path());
    let source = super::registry::source(&inputs, NativeCompanion::Kotlin)
        .expect("the linked services lower")
        .expect("a Kotlin program");
    LinkedPageCheck::assert_once(
        &source,
        &[
            "object Behaviors {",
            "class PageProgramSetup",
            "class WriterService",
            "class ReaderService",
            "private object TachyonService0 {",
            "private object TachyonService1 {",
        ],
    );
}

#[test]
fn two_linked_kotlin_services_share_one_setup_configuration() {
    if !LinkedPageCheck::available("kotlinc", "-version")
        || !LinkedPageCheck::available("java", "-version")
    {
        eprintln!("skipping: Kotlin JVM tools are unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let inputs = LinkedPageCheck::kotlin_inputs(root.path());
    let registry = super::registry::source(&inputs, NativeCompanion::Kotlin)
        .expect("the linked services lower")
        .expect("a Kotlin program");
    let linked = super::android_yon_reach::AndroidYonReach::declared(None).linked(&registry);
    let models = format!("package models\n\n{}", LinkedPageCheck::KOTLIN_FILES[0].1);
    LinkedPageCheck::write(root.path(), "jvm/Models.kt", &models);
    let output = ManagedCapabilityCheck::kotlin_jvm(
        root.path(),
        &linked,
        LinkedPageCheck::KOTLIN_CALLER,
        true,
    );
    println!("{output}");
    LinkedPageCheck::assert_shared(&output);
}

#[test]
fn two_linked_csharp_services_keep_one_copy_of_their_setup_and_client() {
    let root = tempfile::tempdir().expect("project");
    let inputs = LinkedPageCheck::csharp_inputs(root.path());
    let source = super::registry::source(&inputs, NativeCompanion::CSharp)
        .expect("the linked services lower")
        .expect("a C# program");
    LinkedPageCheck::assert_once(
        &source,
        &[
            "public sealed class PageProgramSetup",
            "public sealed class WriterService",
            "public sealed class ReaderService",
            "namespace TachyonServiceCsharp {",
            "namespace TachyonService0 {",
            "namespace TachyonService1 {",
        ],
    );
}

#[test]
fn two_linked_csharp_services_share_one_setup_configuration() {
    if !LinkedPageCheck::available("dotnet", "--version") {
        eprintln!("skipping: dotnet is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let output = LinkedPageCheck::csharp_run(root.path());
    println!("{output}");
    LinkedPageCheck::assert_shared(&output);
}
