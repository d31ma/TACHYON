use super::runtime_assets::RuntimeAssets;
use crate::{BuildOptions, WebCompiler};
use std::path::Path;

struct BuiltPythonApp;

impl BuiltPythonApp {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/manifest.json",
            r#"{"tac":{"python":{"runtime":"pyodide-314.0.7","profile":"offline"}}}"#,
        ),
        (
            "client/services/TallyService.py",
            "@service\nclass TallyService:\n    count: int = 6\n",
        ),
        ("client/pages/tac.html", "<basket-tally />"),
        (
            "client/components/basket/tally/tac.html",
            "<p>{tally.count} in the basket</p>",
        ),
    ];
    const LICENSES: [&'static str; 14] = [
        "Mozilla Public License Version 2.0",
        "https://github.com/pyodide/pyodide/tree/314.0.7",
        "PYTHON SOFTWARE FOUNDATION LICENSE VERSION 2",
        "Licenses and Acknowledgements for Incorporated Software",
        "HACL* Contributors",
        "libffi - Copyright (c) 1996-2022",
        "Jean-loup Gailly and Mark Adler",
        "Julian R Seward",
        "For Zstandard software",
        "liblzma is in the public domain",
        "The author disclaims copyright to this source code.",
        "Emscripten is available under 2 licenses",
        "musl as a whole is licensed under the following standard MIT license",
        "Apache License v2.0 with LLVM Exceptions",
    ];

    fn write(path: &Path, bytes: &[u8]) {
        let parent = path
            .parent()
            .unwrap_or_else(|| panic!("parent of {}", path.display()));
        std::fs::create_dir_all(parent).unwrap_or_else(|error| panic!("folder: {error:?}"));
        std::fs::write(path, bytes).unwrap_or_else(|error| panic!("write: {error:?}"));
    }

    fn stage_runtime(core: &Path) {
        let verified = RuntimeAssets::read(core)
            .unwrap_or_else(|error| panic!("pinned core assets: {error:?}"));
        for (path, bytes) in verified {
            let name = Path::new(&path)
                .file_name()
                .unwrap_or_else(|| panic!("pinned filename"));
            Self::write(&RuntimeAssets::cached_root().join(name), &bytes);
        }
    }

    async fn output(project: &Path) -> std::path::PathBuf {
        for (path, source) in Self::FILES {
            Self::write(&project.join(path), source.as_bytes());
        }
        WebCompiler::build_async(project, &BuildOptions::default())
            .await
            .unwrap_or_else(|error| panic!("Python app build: {error:?}"))
            .output_directory()
            .to_path_buf()
    }
}

#[test]
fn the_runtime_notices_name_every_license_in_the_pinned_runtime() {
    let (path, notices) = RuntimeAssets::notices();
    let text = String::from_utf8(notices).unwrap_or_else(|error| panic!("text: {error:?}"));

    assert_eq!(
        path,
        ".tachyon/services/python/314.0.7/THIRD_PARTY_NOTICES.txt"
    );
    for license in BuiltPythonApp::LICENSES {
        assert!(text.contains(license), "{license}");
    }
}

#[tokio::test]
#[ignore = "requires pinned five-file core assets; set TAC_PYTHON_CORE to the verified local directory and TAC_CACHE_DIR to a scratch cache"]
async fn a_built_python_app_ships_the_runtime_notices_beside_the_runtime() {
    let core = std::env::var_os("TAC_PYTHON_CORE")
        .unwrap_or_else(|| panic!("explicit pinned asset directory"));
    assert!(
        std::env::var_os("TAC_CACHE_DIR").is_some(),
        "set TAC_CACHE_DIR to a scratch cache; the test never writes the global cache"
    );
    BuiltPythonApp::stage_runtime(Path::new(&core));
    let project = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error:?}"));

    let output = BuiltPythonApp::output(project.path()).await;
    let (path, notices) = RuntimeAssets::notices();

    assert!(
        output
            .join(RuntimeAssets::output_name(RuntimeAssets::WASM))
            .is_file()
    );
    assert_eq!(
        std::fs::read(output.join(path)).unwrap_or_else(|error| panic!("notices: {error:?}")),
        notices
    );
}
