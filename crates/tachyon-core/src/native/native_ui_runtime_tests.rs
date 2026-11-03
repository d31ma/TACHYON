#![allow(clippy::expect_used)]

use super::{NativeUiContract, NativeUiGeneration};
use cap_std::{ambient_authority, fs::Dir};
use std::path::Path;

pub(super) struct NativeUiRuntimeFixture {
    root: tempfile::TempDir,
    native_sha256: String,
}

impl NativeUiRuntimeFixture {
    pub(super) const SOURCES: [(&str, &str); 11] = [
        (
            "Failure.swift",
            include_str!("live/NativeUiRuntimeFailure.swift"),
        ),
        (
            "Bounds.swift",
            include_str!("live/NativeUiRuntimeBounds.swift"),
        ),
        ("Json.swift", include_str!("live/NativeUiRuntimeJson.swift")),
        (
            "Contract.swift",
            include_str!("live/NativeUiRuntimeContract.swift"),
        ),
        (
            "Asset.swift",
            include_str!("live/NativeUiRuntimeAsset.swift"),
        ),
        (
            "Descriptor.swift",
            include_str!("live/NativeUiRuntimeDescriptor.swift"),
        ),
        (
            "Directory.swift",
            include_str!("live/NativeUiRuntimeDirectory.swift"),
        ),
        ("File.swift", include_str!("live/NativeUiRuntimeFile.swift")),
        (
            "Capture.swift",
            include_str!("live/NativeUiRuntimeCapture.swift"),
        ),
        (
            "Change.swift",
            include_str!("live/NativeUiRuntimeChange.swift"),
        ),
        (
            "Generation.swift",
            include_str!("live/NativeUiRuntimeGeneration.swift"),
        ),
    ];

    fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().expect("native runtime fixture"),
            native_sha256: "a".repeat(64),
        };
        fixture.write("assets/index.html", b"<main>Installed</main>");
        fixture.write("assets/styles/theme.css", b"main { color: black; }");
        fixture.write("assets/runtime.js", b"globalThis.example = 1;");
        fixture
    }

    fn write(&self, path: &str, bytes: &[u8]) {
        let path = self.root.path().join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, bytes).expect("fixture bytes");
    }

    fn descriptor(&self) -> (Vec<u8>, String) {
        let directory = Dir::open_ambient_dir(self.root.path().join("assets"), ambient_authority())
            .expect("explicit test-owned authority");
        let contract =
            NativeUiContract::macos("org.example.native".to_owned(), self.native_sha256.clone())
                .expect("test installed contract");
        let generation = NativeUiGeneration::capture(&directory, contract).expect("Rust capture");
        (
            generation.descriptor_bytes().expect("descriptor"),
            generation
                .descriptor_sha256()
                .expect("independent native test authority"),
        )
    }

    async fn compile(&self) -> std::path::PathBuf {
        let mut arguments = vec![String::from("swiftc"), String::from("-parse-as-library")];
        for (name, source) in Self::SOURCES {
            self.write(name, source.as_bytes());
            arguments.push(self.root.path().join(name).to_string_lossy().into_owned());
        }
        self.write(
            "Check.swift",
            include_bytes!("live/NativeUiRuntimeCheck.fixture.swift"),
        );
        arguments.push(
            self.root
                .path()
                .join("Check.swift")
                .to_string_lossy()
                .into_owned(),
        );
        let program = self.root.path().join("runtime-check");
        arguments.extend([String::from("-o"), program.to_string_lossy().into_owned()]);
        super::host::run_tool(
            "/usr/bin/xcrun",
            &arguments.iter().map(String::as_str).collect::<Vec<_>>(),
        )
        .await
        .expect("Swift trusted capture compilation");
        program
    }

    async fn execute(&self, program: &Path, mode: &str, descriptor: (&[u8], &str)) {
        self.write("descriptor.json", descriptor.0);
        let assets = self.root.path().join("assets");
        let fingerprint = descriptor.1;
        let descriptor = self.root.path().join("descriptor.json");
        let result = super::host::run_tool(
            &program.to_string_lossy(),
            &[
                mode,
                &assets.to_string_lossy(),
                &descriptor.to_string_lossy(),
                fingerprint,
            ],
        )
        .await
        .expect("Swift fixture execution");
        assert!(result.contains("native-ui-runtime-"), "{result}");
    }
    async fn pair(&self, program: &Path, candidate: &Self, mode: &str) {
        let installed = self.descriptor();
        let proposed = candidate.descriptor();
        self.write("descriptor.json", &installed.0);
        candidate.write("descriptor.json", &proposed.0);
        let installed_root = self.root.path().join("assets");
        let candidate_root = candidate.root.path().join("assets");
        let installed_descriptor = self.root.path().join("descriptor.json");
        let candidate_descriptor = candidate.root.path().join("descriptor.json");
        let result = super::host::run_tool(
            &program.to_string_lossy(),
            &[
                mode,
                &installed_root.to_string_lossy(),
                &installed_descriptor.to_string_lossy(),
                &installed.1,
                &candidate_root.to_string_lossy(),
                &candidate_descriptor.to_string_lossy(),
                &proposed.1,
            ],
        )
        .await
        .expect("Swift compatible/restart execution");
        assert!(
            result.contains("native-ui-runtime-classification-passed"),
            "{result}"
        );
    }
}

#[tokio::test]
async fn native_ui_runtime_validates_rust_identity_vectors_and_retains_captured_bytes() {
    let fixture = NativeUiRuntimeFixture::new();
    fixture.write(
        "assets/unicode/é-🙂-\"-\t-\u{2028}.txt",
        b"unicode path vector",
    );
    let (source, fingerprint) = fixture.descriptor();
    let program = fixture.compile().await;
    fixture
        .execute(&program, "valid", (&source, &fingerprint))
        .await;
    fixture
        .execute(&program, "retained", (&source, &fingerprint))
        .await;
}

#[tokio::test]
async fn native_ui_runtime_refuses_descriptor_trust_schema_and_complete_asset_failures() {
    let fixture = NativeUiRuntimeFixture::new();
    let (source, fingerprint) = fixture.descriptor();
    let program = fixture.compile().await;
    fixture
        .execute(&program, "refuse", (&source, &"b".repeat(64)))
        .await;
    let mut document: serde_json::Value = serde_json::from_slice(&source).expect("document");
    document["unknown"] = serde_json::json!(true);
    let invalid = serde_json::to_vec(&document).expect("unknown field");
    let trusted = super::native_ui_descriptor::NativeUiDescriptor::sha256(&invalid);
    fixture
        .execute(&program, "refuse", (&invalid, &trusted))
        .await;
    document = serde_json::from_slice(&source).expect("original document");
    document["schemaVersion"] = serde_json::json!(2);
    let invalid = serde_json::to_vec(&document).expect("unknown schema");
    let trusted = super::native_ui_descriptor::NativeUiDescriptor::sha256(&invalid);
    fixture
        .execute(&program, "refuse", (&invalid, &trusted))
        .await;
    let duplicate = String::from_utf8(source.clone())
        .expect("UTF-8 descriptor")
        .replacen(
            "\"schemaVersion\":1",
            "\"schemaVersion\":1,\"schemaVersion\":1",
            1,
        )
        .into_bytes();
    let trusted = super::native_ui_descriptor::NativeUiDescriptor::sha256(&duplicate);
    fixture
        .execute(&program, "refuse", (&duplicate, &trusted))
        .await;
    fixture.write("assets/unlisted.txt", b"extra");
    fixture
        .execute(&program, "refuse", (&source, &fingerprint))
        .await;
    std::fs::remove_file(fixture.root.path().join("assets/unlisted.txt")).expect("remove fixture");
    fixture.write("assets/index.html", b"tampered");
    fixture
        .execute(&program, "refuse", (&source, &fingerprint))
        .await;
}

#[tokio::test]
async fn native_ui_runtime_refuses_symlinks_special_files_and_depth_overflow() {
    use std::os::unix::fs::symlink;
    let fixture = NativeUiRuntimeFixture::new();
    let (source, fingerprint) = fixture.descriptor();
    let program = fixture.compile().await;
    let link = fixture.root.path().join("assets/link");
    symlink("index.html", &link).expect("file symlink");
    fixture
        .execute(&program, "refuse", (&source, &fingerprint))
        .await;
    std::fs::remove_file(&link).expect("remove symlink");
    symlink("styles", &link).expect("directory symlink");
    fixture
        .execute(&program, "refuse", (&source, &fingerprint))
        .await;
    std::fs::remove_file(&link).expect("remove symlink");
    super::host::run_tool("/usr/bin/mkfifo", &[&link.to_string_lossy()])
        .await
        .expect("test-owned FIFO");
    fixture
        .execute(&program, "refuse", (&source, &fingerprint))
        .await;
    std::fs::remove_file(&link).expect("remove FIFO");
    let oversized = fixture.root.path().join("assets/oversized.bin");
    std::fs::File::create(&oversized)
        .expect("oversized fixture")
        .set_len(16 * 1024 * 1024 + 1)
        .expect("sparse byte-bound fixture");
    fixture
        .execute(&program, "refuse", (&source, &fingerprint))
        .await;
    std::fs::remove_file(oversized).expect("remove oversized fixture");
    let deep = format!("assets/{}/deep.txt", vec!["d"; 33].join("/"));
    fixture.write(&deep, b"deep");
    fixture
        .execute(&program, "refuse", (&source, &fingerprint))
        .await;
}

#[tokio::test]
async fn native_ui_runtime_classifies_styles_and_rejects_script_and_native_changes() {
    let installed = NativeUiRuntimeFixture::new();
    let mut candidate = NativeUiRuntimeFixture::new();
    let program = installed.compile().await;
    candidate.write("assets/styles/theme.css", b"main { color: blue; }");
    installed.pair(&program, &candidate, "styles").await;
    candidate.write("assets/runtime.js", b"globalThis.example = 2;");
    installed.pair(&program, &candidate, "restart").await;
    candidate.write("assets/runtime.js", b"globalThis.example = 1;");
    candidate.native_sha256 = "b".repeat(64);
    installed.pair(&program, &candidate, "restart").await;
}
