#![allow(clippy::expect_used)]

use super::{
    NativeUiChange, NativeUiContract, NativeUiGeneration, native_ui_descriptor::NativeUiDescriptor,
};
use cap_std::{ambient_authority, fs::Dir};
use std::path::Path;

struct NativeUiFixture;

impl NativeUiFixture {
    fn directory() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("owned generation");
        Self::write(root.path(), "index.html", "<main>Ready</main>");
        Self::write(root.path(), "styles/app.css", "main { color: black; }");
        Self::write(
            root.path(),
            "tachyon-runtime/client.js",
            "export class Client {}",
        );
        root
    }

    fn write(root: &Path, name: &str, source: &str) {
        let path = root.join(name);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("asset");
    }

    fn open(root: &Path) -> Dir {
        Dir::open_ambient_dir(root, ambient_authority()).expect("captured directory authority")
    }

    fn contract() -> NativeUiContract {
        NativeUiContract::macos("org.example.native".to_owned(), "a".repeat(64))
            .expect("installed native fingerprint")
    }

    fn capture(root: &Path) -> NativeUiGeneration {
        NativeUiGeneration::capture(&Self::open(root), Self::contract()).expect("capture")
    }

    fn failure(root: &Path, document: &[u8]) {
        let expected = NativeUiDescriptor::sha256(document);
        assert!(NativeUiGeneration::trusted(&Self::open(root), document, &expected).is_err());
    }
}

#[test]
fn native_ui_generation_verifies_a_trusted_descriptor_and_retains_captured_bytes() {
    let root = NativeUiFixture::directory();
    let installed = NativeUiFixture::capture(root.path());
    let bytes = installed.descriptor_bytes().expect("descriptor");
    let expected = installed
        .descriptor_sha256()
        .expect("external trust record");
    let candidate =
        NativeUiGeneration::trusted(&NativeUiFixture::open(root.path()), &bytes, &expected)
            .expect("verified bytes");
    assert_eq!(candidate.compare(&installed), NativeUiChange::Unchanged);
    NativeUiFixture::write(root.path(), "index.html", "changed after verification");
    assert_eq!(
        candidate.asset("index.html"),
        Some(b"<main>Ready</main>".as_slice())
    );
    assert!(
        NativeUiGeneration::trusted(&NativeUiFixture::open(root.path()), &bytes, &expected)
            .is_err()
    );
}

#[test]
fn native_ui_generation_classifies_styles_without_claiming_activation() {
    let root = NativeUiFixture::directory();
    let installed = NativeUiFixture::capture(root.path());
    NativeUiFixture::write(root.path(), "styles/app.css", "main { color: blue; }");
    let candidate = NativeUiFixture::capture(root.path());
    assert_eq!(candidate.compare(&installed), NativeUiChange::PrepareStyles);
    NativeUiFixture::write(root.path(), "styles/extra.css", "main { display: block; }");
    assert_eq!(
        NativeUiFixture::capture(root.path()).compare(&installed),
        NativeUiChange::PrepareStyles
    );
    std::fs::remove_file(root.path().join("styles/app.css")).expect("remove style");
    assert_eq!(
        NativeUiFixture::capture(root.path()).compare(&installed),
        NativeUiChange::PrepareStyles
    );
}

#[test]
fn native_ui_generation_requires_restart_for_scripts_documents_native_or_grant_changes() {
    for path in [
        "index.html",
        "tachyon-runtime/client.js",
        "native-grants.json",
        "models/codec.wasm",
    ] {
        let root = NativeUiFixture::directory();
        let installed = NativeUiFixture::capture(root.path());
        NativeUiFixture::write(root.path(), path, "changed contract");
        assert_eq!(
            NativeUiFixture::capture(root.path()).compare(&installed),
            NativeUiChange::RestartRequired,
            "{path}"
        );
    }
    let root = NativeUiFixture::directory();
    let installed = NativeUiFixture::capture(root.path());
    let changed = NativeUiContract::macos("org.example.native".to_owned(), "b".repeat(64))
        .expect("changed installed ABI/native/grant fingerprint");
    let candidate = NativeUiGeneration::capture(&NativeUiFixture::open(root.path()), changed)
        .expect("capture changed contract");
    assert_eq!(
        candidate.compare(&installed),
        NativeUiChange::RestartRequired
    );
}

#[test]
fn native_ui_generation_rejects_untrusted_unknown_incomplete_and_noncanonical_descriptors() {
    let root = NativeUiFixture::directory();
    let candidate = NativeUiFixture::capture(root.path());
    let source = candidate.descriptor_bytes().expect("descriptor");
    assert!(
        NativeUiGeneration::trusted(
            &NativeUiFixture::open(root.path()),
            &source,
            &"b".repeat(64)
        )
        .is_err()
    );
    let document: serde_json::Value = serde_json::from_slice(&source).expect("JSON");
    for alteration in [
        "schema",
        "unknown",
        "duplicate",
        "traversal",
        "missing",
        "hash",
        "generation",
        "platform",
    ] {
        let mut changed = document.clone();
        match alteration {
            "schema" => changed["schemaVersion"] = 2.into(),
            "unknown" => changed["downloadUrl"] = "https://example.invalid".into(),
            "duplicate" => {
                let asset = changed["assets"][0].clone();
                changed["assets"]
                    .as_array_mut()
                    .expect("assets")
                    .push(asset);
            }
            "traversal" => changed["assets"][0]["path"] = "../index.html".into(),
            "missing" => {
                changed["assets"].as_array_mut().expect("assets").remove(0);
            }
            "hash" => changed["assets"][0]["sha256"] = "b".repeat(64).into(),
            "generation" => changed["generation"] = "b".repeat(64).into(),
            "platform" => changed["contract"]["target"] = "windows".into(),
            _ => unreachable!(),
        }
        NativeUiFixture::failure(
            root.path(),
            &serde_json::to_vec(&changed).expect("altered JSON"),
        );
    }
    NativeUiFixture::failure(root.path(), &vec![b' '; NativeUiDescriptor::MAX_BYTES + 1]);
    NativeUiFixture::write(root.path(), "extra.html", "unlisted asset");
    NativeUiFixture::failure(root.path(), &source);
}

#[cfg(unix)]
#[test]
fn native_ui_generation_rejects_file_and_directory_symlinks() {
    for directory in [false, true] {
        let root = NativeUiFixture::directory();
        let outside = tempfile::tempdir().expect("outside");
        NativeUiFixture::write(outside.path(), "secret.txt", "outside");
        let target = if directory {
            outside.path().to_owned()
        } else {
            outside.path().join("secret.txt")
        };
        std::os::unix::fs::symlink(target, root.path().join("linked")).expect("symlink");
        assert!(
            NativeUiGeneration::capture(
                &NativeUiFixture::open(root.path()),
                NativeUiFixture::contract()
            )
            .is_err()
        );
    }
}

#[test]
fn native_ui_generation_bounds_asset_bytes_and_directory_depth() {
    let root = NativeUiFixture::directory();
    let file = std::fs::File::create(root.path().join("oversized.css")).expect("file");
    file.set_len(16 * 1024 * 1024 + 1)
        .expect("sparse oversized asset");
    assert!(
        NativeUiGeneration::capture(
            &NativeUiFixture::open(root.path()),
            NativeUiFixture::contract()
        )
        .is_err()
    );
    let root = NativeUiFixture::directory();
    let path = std::iter::repeat_n("nested", 33)
        .collect::<Vec<_>>()
        .join("/");
    std::fs::create_dir_all(root.path().join(path)).expect("deep directory");
    assert!(
        NativeUiGeneration::capture(
            &NativeUiFixture::open(root.path()),
            NativeUiFixture::contract()
        )
        .is_err()
    );
}

#[test]
fn native_ui_generation_rejects_ambiguous_asset_names_and_empty_roots() {
    for name in [
        "percent%20.css",
        "query?.css",
        "fragment#.css",
        "back\\slash.css",
    ] {
        let root = NativeUiFixture::directory();
        NativeUiFixture::write(root.path(), name, "not canonical");
        assert!(
            NativeUiGeneration::capture(
                &NativeUiFixture::open(root.path()),
                NativeUiFixture::contract()
            )
            .is_err(),
            "{name}"
        );
    }
    let root = tempfile::tempdir().expect("empty");
    assert!(
        NativeUiGeneration::capture(
            &NativeUiFixture::open(root.path()),
            NativeUiFixture::contract()
        )
        .is_err()
    );
}

#[test]
fn native_ui_generation_bounds_file_count_and_total_retained_bytes() {
    let root = tempfile::tempdir().expect("file count root");
    for index in 0..4097 {
        std::fs::write(root.path().join(format!("file-{index}.css")), "").expect("small asset");
    }
    assert!(
        NativeUiGeneration::capture(
            &NativeUiFixture::open(root.path()),
            NativeUiFixture::contract()
        )
        .is_err()
    );
    let root = tempfile::tempdir().expect("byte budget root");
    for index in 0..5 {
        let file =
            std::fs::File::create(root.path().join(format!("file-{index}.css"))).expect("file");
        file.set_len(16 * 1024 * 1024)
            .expect("bounded sparse asset");
    }
    assert!(
        NativeUiGeneration::capture(
            &NativeUiFixture::open(root.path()),
            NativeUiFixture::contract()
        )
        .is_err()
    );
}

#[test]
fn native_ui_generation_rejects_invalid_installed_contract_identity() {
    for application in ["", "contains a space", "org/example", "non-ascii-ö"] {
        assert!(NativeUiContract::macos(application.to_owned(), "a".repeat(64)).is_err());
    }
    for fingerprint in ["a".repeat(63), "A".repeat(64), "z".repeat(64)] {
        assert!(NativeUiContract::macos("org.example.app".to_owned(), fingerprint).is_err());
    }
}
