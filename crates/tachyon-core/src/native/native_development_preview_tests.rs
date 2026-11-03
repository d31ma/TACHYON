#![allow(clippy::expect_used)]

use super::DevelopmentOrigin;
use super::compiler::{NativeBuildOptions, NativeCompiler};
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

struct DevelopmentPreviewProject;

impl DevelopmentPreviewProject {
    const PORT: u16 = 4567;
    const MANIFEST: &'static str =
        r#"{"name":"Preview Check","tac":{"yon":{"origin":"https://api.example.com"}}}"#;
    const HOST_SOURCES: [&'static str; 3] =
        ["TachyonHost.swift", "tachyon_host.c", "MainActivity.java"];
    const TARGETS: [NativeTarget; 5] = [
        NativeTarget::Macos,
        NativeTarget::Ios,
        NativeTarget::Android,
        NativeTarget::Linux,
        NativeTarget::Windows,
    ];

    fn project() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("preview project");
        for (relative, source) in [
            ("client/manifest.json", Self::MANIFEST),
            ("client/pages/tac.html", "<home />"),
            ("client/components/home/tac.html", "<main>Preview</main>"),
        ] {
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, source).expect("source");
        }
        root
    }

    fn options(root: &Path, target: NativeTarget) -> NativeBuildOptions {
        NativeBuildOptions {
            output_directory: root.join("dist"),
            target,
            package: false,
            ..NativeBuildOptions::default()
        }
    }

    fn files(folder: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let mut pending = vec![folder.to_path_buf()];
        while let Some(next) = pending.pop() {
            for entry in std::fs::read_dir(&next).expect("folder").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                } else {
                    found.push(path);
                }
            }
        }
        found
    }

    fn host_source(folder: &Path) -> String {
        let host = Self::files(folder)
            .into_iter()
            .find(|path| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| Self::HOST_SOURCES.contains(&name))
            })
            .expect("generated host source");
        std::fs::read_to_string(host).expect("host source")
    }

    fn mentions(folder: &Path, text: &str) -> Vec<PathBuf> {
        Self::files(folder)
            .into_iter()
            .filter(|path| {
                std::fs::read(path).is_ok_and(|bytes| {
                    bytes
                        .windows(text.len())
                        .any(|window| window == text.as_bytes())
                })
            })
            .collect()
    }
}

#[tokio::test]
async fn a_preview_host_fetches_its_pages_from_the_development_server_and_a_plain_build_does_not() {
    for target in DevelopmentPreviewProject::TARGETS {
        let root = DevelopmentPreviewProject::project();
        let development = DevelopmentOrigin::new(target, DevelopmentPreviewProject::PORT);
        let previewed = NativeCompiler::build_async(
            root.path(),
            &NativeBuildOptions {
                development_origin: Some(development.clone()),
                ..DevelopmentPreviewProject::options(&root.path().join("preview"), target)
            },
        )
        .await
        .unwrap_or_else(|failure| panic!("{target:?} preview build: {failure}"));
        let host = DevelopmentPreviewProject::host_source(previewed.output_directory());
        assert!(
            host.contains(&format!("\"{}\"", development.origin())),
            "{target:?} host names the development server"
        );
        assert!(
            !host.contains("__DEVELOPMENT_"),
            "{target:?} placeholders filled"
        );
        assert!(
            previewed
                .notes()
                .iter()
                .any(|note| note.contains("https://api.example.com")),
            "{target:?} says the declared Yon origin is replaced: {:?}",
            previewed.notes()
        );

        let plain = NativeCompiler::build_async(
            root.path(),
            &DevelopmentPreviewProject::options(root.path(), target),
        )
        .await
        .unwrap_or_else(|failure| panic!("{target:?} plain build: {failure}"));
        let host = DevelopmentPreviewProject::host_source(plain.output_directory());
        assert!(
            !host.contains("__DEVELOPMENT_"),
            "{target:?} placeholders filled"
        );
        let port = format!(":{}", DevelopmentPreviewProject::PORT);
        assert_eq!(
            DevelopmentPreviewProject::mentions(plain.output_directory(), &port),
            Vec::<PathBuf>::new(),
            "{target:?} plain build holds no development origin"
        );
        assert_eq!(
            std::fs::read_to_string(root.path().join("client/manifest.json")).expect("manifest"),
            DevelopmentPreviewProject::MANIFEST,
            "{target:?} preview leaves the authored manifest alone"
        );
    }
}

#[tokio::test]
async fn a_release_build_refuses_the_development_server() {
    let root = DevelopmentPreviewProject::project();
    for target in DevelopmentPreviewProject::TARGETS {
        let failure = NativeCompiler::build_async(
            root.path(),
            &NativeBuildOptions {
                release: true,
                development_origin: Some(DevelopmentOrigin::new(
                    target,
                    DevelopmentPreviewProject::PORT,
                )),
                ..DevelopmentPreviewProject::options(root.path(), target)
            },
        )
        .await
        .expect_err("a release never reaches the development server");
        assert_eq!(failure.diagnostics()[0].code.number(), 1601, "{target:?}");
        assert!(
            !root.path().join("dist").exists(),
            "{target:?} refused before writing output"
        );
    }
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "Requires the x86_64-w64-mingw32-gcc cross compiler and TAC_WEBVIEW2_SDK; compiles the preview Windows host against the Win32, WinHTTP and WebView2 headers, not MSVC"]
async fn the_preview_windows_host_compiles_against_the_win32_and_webview2_headers() {
    use super::windows_title_bar_tests::WindowsTitleBarTests;
    let root = DevelopmentPreviewProject::project();
    let previewed = NativeCompiler::build_async(
        root.path(),
        &NativeBuildOptions {
            development_origin: Some(DevelopmentOrigin::new(
                NativeTarget::Windows,
                DevelopmentPreviewProject::PORT,
            )),
            ..DevelopmentPreviewProject::options(root.path(), NativeTarget::Windows)
        },
    )
    .await
    .expect("preview build");
    let source = root.path().join("preview-host.c");
    std::fs::write(
        &source,
        DevelopmentPreviewProject::host_source(previewed.output_directory()),
    )
    .expect("source");
    WindowsTitleBarTests::success(
        std::process::Command::new(WindowsTitleBarTests::MINGW)
            .args(WindowsTitleBarTests::WARNINGS)
            .args([
                "-Wno-unused-parameter",
                "-Wno-unused-function",
                "-Wno-cast-function-type",
                "-Wno-unknown-pragmas",
            ])
            .args(WindowsTitleBarTests::headers(root.path()))
            .arg("-c")
            .arg(&source)
            .arg("-o")
            .arg(root.path().join("preview-host.o")),
    );
}
