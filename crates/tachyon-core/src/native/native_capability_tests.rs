#![allow(clippy::expect_used)]

use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;

struct NativeCapabilityCheck;

impl NativeCapabilityCheck {
    const SERVICE: &str = "@Service\n@NeedsAll\nfinal class ImportService {\n    private let filePicker: FilePicker\n    private let windowControls: WindowControls\n    func choose() -> String { filePicker.open().status.rawValue }\n    func allowed() -> Bool { windowControls.granted(\"minimize\") }\n}\n";
    const CALLER: &str = r#"import Foundation
@main struct Caller {
    static func main() {
        NativeCapabilities.call = { capability, _ in
            capability == "window.granted" ? "{\"granted\":true}" : "{\"status\":\"cancelled\"}"
        }
        let owner = "6F9619FF-8B86-D011-B42D-00C04FC964FF"
        let prefix = "{\"route\":\"/\",\"owner\":\"" + owner + "\",\"service\":\"import\","
        print(tacNativeInvoke(prefix + "\"op\":\"init\"}"))
        print(tacNativeInvoke(prefix + "\"op\":\"call\",\"name\":\"choose\",\"args\":[]}"))
        print(tacNativeInvoke(prefix + "\"op\":\"call\",\"name\":\"allowed\",\"args\":[]}"))
        let retained = NativeCapabilities.create(owner) { NativeCapabilities.filePicker() }
        print(tacNativeInvoke(prefix + "\"op\":\"dispose\"}"))
        precondition(retained.open().status == .disposed)
    }
}
"#;

    fn stage(root: &std::path::Path) -> std::path::PathBuf {
        let source = root.join("client/services/ImportService.swift");
        std::fs::create_dir_all(source.parent().expect("parent")).expect("directory");
        std::fs::write(&source, Self::SERVICE).expect("source");
        let input = NativeCompanionInput {
            snapshot_root: root.to_path_buf(),
            language: NativeCompanion::Swift,
            source,
            service: String::from("import"),
            class: String::from("ImportService"),
            web_assembly: false,
        };
        super::macos::stage_swift_companion(&[input], root)
            .expect("stage")
            .expect("companion")
    }

    async fn run() -> String {
        let root = tempfile::tempdir().expect("project");
        let staged = Self::stage(root.path());
        let json = root.path().join("json.swift");
        std::fs::write(&json, super::macos::MacOsHostGenerator::APPLE_JSON_REQUEST).expect("json");
        let caller = root.path().join("caller.swift");
        std::fs::write(&caller, Self::CALLER).expect("caller");
        let program = root.path().join("caller");
        super::host::run_tool(
            "/usr/bin/xcrun",
            &[
                "swiftc",
                "-parse-as-library",
                &staged.to_string_lossy(),
                &json.to_string_lossy(),
                &caller.to_string_lossy(),
                "-o",
                &program.to_string_lossy(),
            ],
        )
        .await
        .expect("compile");
        super::host::run_tool(&program.to_string_lossy(), &[])
            .await
            .expect("execute")
    }

    fn fake_unit(extension: &str) -> bool {
        let root = tempfile::tempdir().expect("project");
        let folder = root.path().join("client/tests");
        let services = root.path().join("client/services");
        std::fs::create_dir_all(&folder).expect("tests");
        std::fs::create_dir_all(&services).expect("services");
        let service = "@Service\nexport default class ImportService {\n    /** @type {TacFilePicker} */\n    @Needs\n    filePicker;\n    /** @returns {Promise<string>} */\n    async choose() { return (await this.filePicker.open()).status; }\n}\n";
        let setup = "@TestSetup\nclass ImportTestSetup {\n    /** @returns {object} */\n    static configure() { return {}; }\n}\n";
        let unit = "@Test(ImportTestSetup)\nexport default class ImportServiceTest {\n    /** @type {TacFilePicker} */\n    @Fake\n    filePicker;\n    /** @returns {Promise<void>} */\n    async testGetsTheFakePicker() { Assert.equals('cancelled', await this.prepare().choose()); }\n    /** @returns {ImportService} */\n    prepare() { this.filePicker = { open: async () => ({ status: 'cancelled' }) }; return this.construct(ImportService); }\n}\n";
        std::fs::write(services.join(format!("ImportService.{extension}")), service)
            .expect("service");
        std::fs::write(folder.join(format!("ImportTestSetup.{extension}")), setup).expect("setup");
        std::fs::write(folder.join(format!("ImportServiceTest.{extension}")), unit).expect("test");
        let relative = format!("client/tests/ImportServiceTest.{extension}");
        let report = crate::client_tests::run(root.path(), std::path::Path::new(&relative))
            .expect("test unit");
        report.cases.len() == 1 && report.cases[0].passed
    }

    async fn host_compiles() -> String {
        let root = tempfile::tempdir().expect("project");
        let application =
            super::config::NativeApplication::discover(root.path()).expect("application");
        let index = super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        };
        let staged = Self::stage(root.path());
        let host = super::macos::swift_source(
            &application,
            super::macos::companion_call(Some(&staged), None),
            &index,
        );
        let source = root.path().join("Host.swift");
        std::fs::write(&source, host).expect("host");
        super::host::run_tool(
            "/usr/bin/xcrun",
            &[
                "swiftc",
                "-parse-as-library",
                "-typecheck",
                &source.to_string_lossy(),
                &staged.to_string_lossy(),
            ],
        )
        .await
        .expect("compile host")
    }
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn a_swift_service_receives_fake_capabilities_in_its_activation() {
    assert!(
        NativeCapabilityCheck::run()
            .await
            .contains("\"value\":\"cancelled\"")
    );
}

#[tokio::test]
#[cfg(target_os = "macos")]
async fn a_generated_macos_file_picker_host_compiles() {
    assert!(NativeCapabilityCheck::host_compiles().await.is_empty());
}

#[test]
fn a_javascript_test_setup_can_replace_the_file_picker_provider() {
    assert!(NativeCapabilityCheck::fake_unit("js"));
}

#[test]
fn a_typescript_test_setup_can_replace_the_file_picker_provider() {
    assert!(NativeCapabilityCheck::fake_unit("ts"));
}
