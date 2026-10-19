#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;

struct FilePickerPermissionCheck {
    root: PathBuf,
}

impl FilePickerPermissionCheck {
    const SERVICE: &str =
        include_str!("../../../../scripts/native/FilePickerPermissionService.swift");
    const CALLER: &str = include_str!("../../../../scripts/native/FilePickerPermissionCheck.swift");
    const SELECTOR: &str =
        include_str!("../../../../scripts/native/FilePickerPermissionSelect.applescript");
    const OUTPUT_LIMIT: usize = 65_536;

    fn new() -> Self {
        let output = std::env::var_os("TAC_NATIVE_PICKER_OUT")
            .expect("Set TAC_NATIVE_PICKER_OUT to an absolute retained evidence directory");
        let output = PathBuf::from(output);
        assert!(
            output.is_absolute(),
            "The evidence directory must be absolute"
        );
        std::fs::create_dir_all(&output).expect("evidence directory");
        let root = tempfile::Builder::new()
            .prefix("picker-grant-")
            .tempdir_in(output)
            .expect("fresh owned evidence directory")
            .keep();
        println!("Retained picker grant evidence: {}", root.display());
        Self { root }
    }

    async fn run(&self) {
        let companion = self.stage();
        self.host(&companion);
        self.compile(&companion).await;
        self.exercise().await;
    }

    fn stage(&self) -> PathBuf {
        let folder = self.root.join("client/services");
        std::fs::create_dir_all(&folder).expect("service directory");
        let source = folder.join("ImportService.swift");
        std::fs::write(&source, Self::SERVICE).expect("authored service");
        std::fs::write(
            self.root.join("client/manifest.json"),
            r#"{"name":"Picker Grant Check","tac":{"nativeCapabilities":["filePicker"]}}"#,
        )
        .expect("normal manifest grant");
        let input = super::registry::NativeCompanionInput {
            snapshot_root: self.root.clone(),
            language: crate::project::NativeCompanion::Swift,
            source,
            service: String::from("import"),
            class: String::from("ImportService"),
            web_assembly: false,
            application_provider: false,
        };
        super::macos::stage_swift_companion(&[input], &self.root, None)
            .expect("stage normal service")
            .expect("generated companion")
    }

    fn host(&self, companion: &Path) {
        let application = super::config::NativeApplication::discover(&self.root)
            .expect("normal manifest discovery");
        let index = super::routes::NativeRouteIndex {
            contract_version: 2,
            entry_route: String::from("/"),
            entry_document: String::from("index.html"),
            routes: vec![super::routes::NativeRouteIndexEntry {
                route: String::from("/"),
                document: String::from("index.html"),
            }],
        };
        let original = super::macos::swift_source(
            &application,
            super::macos::companion_call(Some(companion), None),
            &index,
        );
        self.write_host(&original);
    }

    fn write_host(&self, original: &str) {
        let grant = Self::startup_line(original, "MacOsFilePicker.shared.granted = ");
        let registration = Self::startup_line(original, "NativeCapabilities.call = ");
        let caller = Self::CALLER
            .replace("__INITIAL_GRANT__", grant)
            .replace("__REGISTER__", registration);
        let (body, _) = original
            .rsplit_once("\n@main\n")
            .expect("only replace generated trailing entry point");
        std::fs::write(self.root.join("GeneratedHost.swift"), original)
            .expect("retain unchanged generated host");
        std::fs::write(
            self.root.join("PickerHost.swift"),
            format!("{body}\n{caller}"),
        )
        .expect("owned entry point in same adapter file");
    }

    fn startup_line<'a>(source: &'a str, prefix: &str) -> &'a str {
        source
            .lines()
            .map(str::trim)
            .find(|line| line.starts_with(prefix))
            .expect("generated manifest/adapter startup line")
    }

    async fn compile(&self, companion: &Path) {
        let mut command = Command::new("/usr/bin/xcrun");
        command.args([
            "swiftc",
            "-parse-as-library",
            "-framework",
            "AppKit",
            "-framework",
            "WebKit",
        ]);
        command.arg(self.root.join("PickerHost.swift"));
        command
            .arg(companion)
            .arg("-o")
            .arg(self.root.join("picker-check"));
        let result =
            crate::external_command::run(&mut command, Duration::from_mins(1), Self::OUTPUT_LIMIT)
                .await;
        self.receipt("compile", result);
    }

    async fn exercise(&self) {
        let selected = self.root.join("selected.txt");
        std::fs::write(&selected, "selected through generated service\n")
            .expect("owned selected file");
        let script = self.root.join("selection.applescript");
        std::fs::write(&script, Self::SELECTOR).expect("owned PID selector");
        let mut picker = Command::new(self.root.join("picker-check"));
        picker.arg(&self.root);
        let mut selector = Command::new("/usr/bin/osascript");
        selector.arg(script).arg(&self.root).arg(selected);
        let (native, ui) = tokio::join!(
            crate::external_command::run(&mut picker, Duration::from_secs(45), Self::OUTPUT_LIMIT),
            crate::external_command::run(
                &mut selector,
                Duration::from_secs(30),
                Self::OUTPUT_LIMIT
            ),
        );
        self.save("native", &native);
        self.save("ui", &ui);
        self.receipt("ui", ui);
        let native = self.receipt("native", native);
        assert!(native.contains("regrant rejected old handle; activation disposed"));
    }

    fn receipt(
        &self,
        label: &str,
        result: Result<crate::external_command::ToolOutput, crate::external_command::ToolError>,
    ) -> String {
        self.save(label, &result);
        let output = result.expect("supervised fixture tool");
        assert!(
            output.status.success(),
            "{label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        println!(
            "{label}: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    fn save(
        &self,
        label: &str,
        result: &Result<crate::external_command::ToolOutput, crate::external_command::ToolError>,
    ) {
        let receipt = Self::tool_receipt(result);
        std::fs::write(self.root.join(format!("{label}.log")), receipt)
            .expect("retained tool receipt");
    }

    fn tool_receipt(
        result: &Result<crate::external_command::ToolOutput, crate::external_command::ToolError>,
    ) -> String {
        match result {
            Ok(output) => format!(
                "status: {}\nstdout:\n{}\nstderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ),
            Err(error) => format!("supervisor failure: {error}\n"),
        }
    }
}

#[tokio::test]
#[ignore = "Opens a real AppKit picker; needs TAC_NATIVE_PICKER_OUT and existing Accessibility access"]
async fn a_selected_file_handle_is_revoked_through_a_swift_service() {
    FilePickerPermissionCheck::new().run().await;
}
