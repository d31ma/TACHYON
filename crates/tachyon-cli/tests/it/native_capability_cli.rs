#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

struct PickerProject {
    root: tempfile::TempDir,
}

impl PickerProject {
    const MANIFEST: &'static str = r#"{"name":"Picker","start_url":"/","tac":{"nativeCapabilities":["filePicker"],"application":{"id":"dev.tachyon.picker","version":"0.0.1"},"visibility":{"check":false}}}"#;
    const PAGE: &'static str = "<picker-panel hydrate=\"load\" />";
    const PANEL: &'static str = "<main><button type=\"button\" on:click=\"picker.check()\">Check</button><p>{picker.status}</p></main>";
    const KOTLIN: &'static str = "@Service\n@NeedsAll\nclass PickerService {\n    private val filePicker: FilePicker\n    var status: String = \"\"\n\n    fun check() {\n        status = describe(filePicker.available())\n    }\n\n    private fun describe(available: Boolean): String {\n        return if (available) \"available\" else \"unsupported\"\n    }\n}\n";
    const SWIFT: &'static str = "@Service\n@NeedsAll\nfinal class PickerService {\n    private let filePicker: FilePicker\n    var status: String = \"\"\n\n    func check() {\n        status = describe(filePicker.available())\n    }\n\n    private func describe(_ available: Bool) -> String {\n        return available ? \"available\" : \"unsupported\"\n    }\n}\n";
    const CSHARP: &'static str = "[Service]\n[NeedsAll]\npublic class PickerService\n{\n    private readonly FilePicker filePicker;\n    public string Status = \"\";\n\n    public void Check()\n    {\n        Status = Describe(filePicker.Available());\n    }\n\n    private string Describe(bool available)\n    {\n        return available ? \"available\" : \"unsupported\";\n    }\n}\n";
    const LUA: &'static str = "-- @Service\n---@class PickerService\nPickerService = {\n  -- @Needs\n  ---@type WindowControls\n  window_controls = nil,\n  ---@type string\n  status = \"\",\n}\n\n---@return nil\nfunction PickerService:check()\n  self.window_controls:set_title(\"Report\")\nend\n";

    fn new(extension: &str, service: &str) -> Self {
        let root = tempfile::tempdir().expect("project");
        let files = [
            ("client/manifest.json", Self::MANIFEST),
            ("client/pages/tac.html", Self::PAGE),
            ("client/components/picker/panel/tac.html", Self::PANEL),
        ];
        for (relative, contents) in files {
            Self::write(&root.path().join(relative), contents);
        }
        let service_path = format!("client/services/PickerService.{extension}");
        Self::write(&root.path().join(service_path), service);
        Self { root }
    }

    fn write(path: &Path, contents: &str) {
        fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directory");
        fs::write(path, contents).expect("fixture source");
    }

    fn bundle(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ty"))
            .arg("bundle")
            .arg(self.root.path())
            .args(arguments)
            .output()
            .expect("the ty process should start")
    }

    fn stderr(output: &Output) -> String {
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    fn assert_bundles(&self, language: &str, arguments: &[&str]) {
        let output = self.bundle(arguments);
        let message = Self::stderr(&output);
        assert!(
            output.status.success(),
            "{language} {arguments:?}: {message}"
        );
    }

    fn has_environment(name: &str) -> bool {
        let ready = std::env::var_os(name).is_some_and(|value| Path::new(&value).exists());
        if !ready {
            eprintln!("skipping: {name} is not set");
        }
        ready
    }

    fn has_dotnet_web_workload() -> bool {
        let listed = Command::new("dotnet").args(["workload", "list"]).output();
        let ready = listed
            .is_ok_and(|output| String::from_utf8_lossy(&output.stdout).contains("wasm-tools"));
        if !ready {
            eprintln!("skipping: the .NET wasm-tools workload is unavailable");
        }
        ready
    }
}

#[test]
fn a_kotlin_needs_all_file_picker_bundles_for_android() {
    let project = PickerProject::new("kt", PickerProject::KOTLIN);
    project.assert_bundles("kt", &["--target", "android", "--native"]);
}

#[test]
fn a_kotlin_needs_all_file_picker_runs_in_the_page_as_unsupported() {
    if !PickerProject::has_environment("TAC_KOTLIN_WASM_STDLIB") {
        return;
    }
    let project = PickerProject::new("kt", PickerProject::KOTLIN);
    project.assert_bundles("kt", &[]);
}

#[test]
fn a_swift_needs_all_file_picker_runs_in_the_page_as_unsupported() {
    if !PickerProject::has_environment("TAC_SWIFT_WASM_SDK") {
        return;
    }
    let project = PickerProject::new("swift", PickerProject::SWIFT);
    project.assert_bundles("swift", &[]);
    project.assert_bundles("swift", &["--target", "android", "--native"]);
}

#[test]
fn a_csharp_needs_all_file_picker_runs_in_the_page_as_unsupported() {
    if !PickerProject::has_dotnet_web_workload() {
        return;
    }
    let project = PickerProject::new("cs", PickerProject::CSHARP);
    project.assert_bundles("cs", &[]);
}

#[test]
fn a_lua_window_controls_field_is_refused_at_build_time() {
    let project = PickerProject::new("lua", PickerProject::LUA);
    let output = project.bundle(&[]);
    let message = PickerProject::stderr(&output);
    assert!(!output.status.success(), "{message}");
    assert!(message.contains("TY2015"), "{message}");
    assert!(
        message.contains("Lua services do not support WindowControls"),
        "{message}"
    );
}
