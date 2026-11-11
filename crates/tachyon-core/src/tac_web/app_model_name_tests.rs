#![allow(clippy::expect_used)]

use super::{BuildJobs, WebLanguage, build_all};
use crate::node_driver::NodeDriver;
use std::path::Path;
use std::process::Command;

struct ShadowingApp {
    extension: &'static str,
    language: WebLanguage,
    model: &'static str,
    service: &'static str,
    profile: crate::BuildProfile,
    fields: &'static [&'static str],
}

impl ShadowingApp {
    const DEADLINE: std::time::Duration = std::time::Duration::from_mins(2);
    const EVENTS_RUNTIME: &str = concat!(
        include_str!("../template/application-events.js"),
        "\nexport default TacApplicationEvents\n"
    );
    const WEB_RUNTIME: &str = include_str!("../template/tac-web.js");

    const DRIVER: &str = "import { readFile } from 'node:fs/promises'
globalThis.fetch = async (url) => new Response(await readFile(url))
;(0, eval)(await readFile('./tac-web.js', 'utf8'))
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: NoteService, __tachyon_event_plan__: plan } = await import('./NoteService.js')
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
const note = new NoteService({})
bus.attach(note, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(note)
bus.publish('UploadFinished', { note: 'a-7' })
await new Promise((resolve) => setTimeout(resolve, 0))
console.log(JSON.stringify({ done: note.done, fields: Object.keys(plan.types[0].listeners[0].models?.UploadFinished?.fields ?? {}) }))
process.exit(0)
";

    const RUST: Self = Self {
        extension: "rs",
        language: WebLanguage::Rust,
        model: "#[derive(Clone, Debug, Default)]\npub struct UploadFinished {\n    pub note: String,\n}\n",
        service: "use crate::models::UploadFinished;\n\n#[Service]\n#[derive(Default)]\npub struct NoteService {\n    pub done: String,\n}\n\n\
             impl NoteService {\n    #[OnEvent]\n    pub fn finished(&mut self, event: UploadFinished) {\n        self.done = event.note;\n    }\n}\n",
        profile: crate::BuildProfile::Release,
        fields: &["note"],
    };

    const GO: Self = Self {
        extension: "go",
        language: WebLanguage::Go,
        model: "type UploadFinished struct {\n\tNote string `json:\"note\"`\n}\n",
        service: "// @Service\ntype NoteService struct {\n\tDone string\n}\n\n\
             // @OnEvent\nfunc (service *NoteService) Finished(event UploadFinished) {\n\tservice.Done = event.Note\n}\n",
        profile: crate::BuildProfile::Release,
        fields: &["note"],
    };

    const CSHARP: Self = Self {
        extension: "cs",
        language: WebLanguage::CSharp,
        model: "public record UploadFinished(string Note);\n",
        service: "using Models;\n\n[Service]\npublic class NoteService\n{\n    public string Done = \"\";\n\n    \
             [OnEvent]\n    public void Finished(UploadFinished finished)\n    {\n        Done = finished.Note;\n    }\n}\n",
        profile: crate::BuildProfile::Development,
        fields: &["note"],
    };

    const FSHARP: Self = Self {
        extension: "fs",
        language: WebLanguage::CSharp,
        model: "type UploadFinished = { Note: string }\n",
        service: "open Models\n\n[<Service>]\ntype NoteService() =\n    member val Done : string = \"\" with get, set\n\n    \
             [<OnEvent>]\n    member this.Finished(finished: UploadFinished) : unit =\n        this.Done <- finished.Note\n",
        profile: crate::BuildProfile::Development,
        fields: &["note"],
    };

    const RUBY: Self = Self {
        extension: "rb",
        language: WebLanguage::Ruby,
        model: "# @!attribute [r] note\n#   @return [String]\nUploadFinished = Data.define(:note)\n",
        service: "#[Service]\nclass NoteService\n  attr_accessor :done\n\n  def initialize\n    @done = \"\"\n  end\n\n  \
             # @param finished [UploadFinished]\n  #[OnEvent]\n  def finished(finished)\n    @done = finished.note\n  end\nend\n",
        profile: crate::BuildProfile::Release,
        fields: &["note"],
    };

    const LUA: Self = Self {
        extension: "lua",
        language: WebLanguage::Lua,
        model: "---@class UploadFinished\n---@field note string\nUploadFinished = {}\n",
        service: "-- @Service\n---@class NoteService\nNoteService = {\n  ---@type string\n  done = \"\",\n}\n\n\
             ---@param finished UploadFinished\n---@return nil\n-- @OnEvent\nfunction NoteService:finished(finished)\n  self.done = finished.note\nend\n",
        profile: crate::BuildProfile::Release,
        fields: &["note"],
    };

    const SWIFT: Self = Self {
        extension: "swift",
        language: WebLanguage::Swift,
        model: "struct UploadFinished {\n    let note: String\n}\n",
        service: "@Service\nfinal class NoteService {\n    var done: String = \"\"\n\n    \
             @OnEvent\n    func finished(_ finished: UploadFinished) {\n        done = finished.note\n    }\n}\n",
        profile: crate::BuildProfile::Development,
        fields: &["note"],
    };

    const KOTLIN: Self = Self {
        extension: "kt",
        language: WebLanguage::Kotlin,
        model: "data class UploadFinished(val note: String)\n",
        service: "import models.UploadFinished\n\n@Service\nclass NoteService {\n    var done: String = \"\"\n\n    \
             @OnEvent\n    fun finished(finished: UploadFinished) {\n        done = finished.note\n    }\n}\n",
        profile: crate::BuildProfile::Release,
        fields: &["note"],
    };

    const JAVA: Self = Self {
        extension: "java",
        language: WebLanguage::Java,
        model: "record UploadFinished(String note) {}\n",
        service: "import models.UploadFinished;\n\n@Service\nclass NoteService {\n    String done = \"\";\n\n    \
             @OnEvent\n    void finished(UploadFinished finished) {\n        done = finished.note();\n    }\n}\n",
        profile: crate::BuildProfile::Development,
        fields: &["note"],
    };

    const CPP: Self = Self {
        extension: "hpp",
        language: WebLanguage::Cpp,
        model: "#pragma once\n#include <string>\n\nstruct UploadFinished {\n    std::string note;\n};\n",
        service: "#pragma once\n#include <string>\n\nclass [[Service]] NoteService {\npublic:\n    std::string done;\n\n    \
             [[OnEvent]] void finished(const UploadFinished& finished) { done = finished.note; }\n};\n",
        profile: crate::BuildProfile::Release,
        fields: &[],
    };

    fn write(root: &Path, relative: &str, text: &str) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(&path, text).expect("write");
        path
    }

    fn write_sources(&self, root: &Path) -> std::path::PathBuf {
        let extension = self.extension;
        Self::write(
            root,
            &format!("client/models/UploadFinished.{extension}"),
            self.model,
        );
        Self::write(
            root,
            &format!("client/services/NoteService.{extension}"),
            self.service,
        )
    }

    async fn publish(&self, root: &Path) {
        let path = self.write_sources(root);
        let services = [(
            String::from("note"),
            String::from("NoteService"),
            self.language,
            path,
        )];
        let published = build_all(root, &services, &BuildJobs::new(1), self.profile)
            .await
            .expect("an app model named like a built-in event builds");
        for (name, bytes) in published.files {
            let path = root.join(&name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("write");
        }
    }

    fn report(root: &Path) -> serde_json::Value {
        Self::write(root, "application-events.mjs", Self::EVENTS_RUNTIME);
        Self::write(root, "tac-web.js", Self::WEB_RUNTIME);
        Self::write(root, "drive.mjs", Self::DRIVER);
        let mut command = NodeDriver::command();
        command.arg("drive.mjs").current_dir(root);
        let output = NodeDriver::output_within(&mut command, Self::DEADLINE)
            .expect("node drive.mjs within its deadline");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }

    async fn assert_own_model_wins(&self) {
        let root = tempfile::tempdir().expect("project");
        self.publish(root.path()).await;
        let report = Self::report(root.path());
        assert_eq!(
            report,
            serde_json::json!({ "done": "a-7", "fields": self.fields }),
            "{report}"
        );
    }

    fn answers(program: &str, arguments: &[&str]) -> bool {
        Command::new(program)
            .args(arguments)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn rust_ready() -> bool {
        let target = crate::native::RustCompanion::WEB_TARGET;
        Self::answers("rustc", &["--print", "target-libdir", "--target", target])
    }

    fn dotnet_ready() -> bool {
        Command::new("dotnet")
            .args(["workload", "list"])
            .output()
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"))
    }

    fn skipped(ready: bool, tool: &str) -> bool {
        let skip = !(ready && NodeDriver::available());
        if skip {
            eprintln!("skipping: node or {tool} is unavailable");
        }
        skip
    }
}

#[tokio::test]
async fn a_rust_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(ShadowingApp::rust_ready(), "the wasm32 Rust target") {
        return;
    }
    ShadowingApp::RUST.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_go_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(ShadowingApp::answers("go", &["version"]), "go") {
        return;
    }
    ShadowingApp::GO.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_csharp_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(ShadowingApp::dotnet_ready(), "the .NET wasm-tools workload") {
        return;
    }
    ShadowingApp::CSHARP.assert_own_model_wins().await;
}

#[tokio::test]
async fn an_fsharp_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(ShadowingApp::dotnet_ready(), "the .NET wasm-tools workload") {
        return;
    }
    ShadowingApp::FSHARP.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_ruby_app_model_named_like_a_built_in_event_wins() {
    let ready = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason().is_none();
    if ShadowingApp::skipped(ready, "the Ruby runtime") {
        return;
    }
    ShadowingApp::RUBY.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_lua_app_model_named_like_a_built_in_event_wins() {
    let ready = crate::tac_lua::LuaRuntimeBuild::unavailable_reason().is_none();
    if ShadowingApp::skipped(ready, "the Lua runtime") {
        return;
    }
    ShadowingApp::LUA.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_swift_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(
        super::swift_tests::swift_web_ready(),
        "the Swift WebAssembly SDK",
    ) {
        return;
    }
    ShadowingApp::SWIFT.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_kotlin_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(super::kotlin_tests::kotlin_web_ready(), "kotlinc-js") {
        return;
    }
    ShadowingApp::KOTLIN.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_java_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(super::java_tests::java_web_ready(), "TeaVM") {
        return;
    }
    ShadowingApp::JAVA.assert_own_model_wins().await;
}

#[tokio::test]
async fn a_cpp_app_model_named_like_a_built_in_event_wins() {
    let ready = crate::program_probe::ProgramProbe::answers("em++", "--version");
    if ShadowingApp::skipped(ready, "em++") {
        return;
    }
    ShadowingApp::CPP.assert_own_model_wins().await;
}

struct DartShadowingApp;

impl DartShadowingApp {
    const MODEL: &str =
        "class UploadFinished {\n  final String note;\n\n  const UploadFinished(this.note);\n}\n";
    const SERVICE: &str = "import 'models/UploadFinished.dart';\n\n@Service()\nclass NoteService {\n  String done = '';\n\n  \
         @OnEvent\n  void finished(UploadFinished finished) {\n    done = finished.note;\n  }\n}\n";

    async fn publish(root: &Path) {
        ShadowingApp::write(root, "client/models/UploadFinished.dart", Self::MODEL);
        let path = ShadowingApp::write(root, "client/services/NoteService.dart", Self::SERVICE);
        let services = [("NoteService", path.as_path())];
        let instances = std::collections::BTreeSet::from([String::from("note")]);
        let modules =
            crate::tac_dart::build_all(root, &services, &instances, crate::BuildProfile::Release)
                .await
                .expect("an app model named like a built-in event builds");
        for (name, bytes) in modules {
            let path = root.join(&name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, bytes).expect("write");
        }
        ShadowingApp::write(root, "package.json", "{\"type\":\"module\"}");
    }
}

#[tokio::test]
async fn a_dart_app_model_named_like_a_built_in_event_wins() {
    if ShadowingApp::skipped(ShadowingApp::answers("dart", &["--version"]), "dart") {
        return;
    }
    let root = tempfile::tempdir().expect("project");
    DartShadowingApp::publish(root.path()).await;
    let report = ShadowingApp::report(root.path());
    assert_eq!(
        report,
        serde_json::json!({ "done": "a-7", "fields": ["note"] }),
        "{report}"
    );
}
