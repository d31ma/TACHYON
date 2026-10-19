#![allow(clippy::expect_used)]

use super::{NativeBuildOptions, NativeCompiler};
use std::fs;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

struct LinuxPackageTests {
    workspace: tempfile::TempDir,
}

// The package runs are Linux-only; elsewhere only the version check uses these helpers.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
impl LinuxPackageTests {
    const MANIFEST: &'static str = r#"{"name":"Package Gate","description":"Checks the Linux packages.","start_url":"/","icons":[{"src":"/shared/icon-256.png","sizes":"256x256","type":"image/png"}],"tac":{"application":{"id":"dev.tachyon.package-gate","version":"0.0.1","build":7}}}"#;
    const FILES: [(&'static str, &'static str); 3] = [
        ("client/pages/tac.html", "<gate-home />"),
        (
            "client/components/gate/home/tac.html",
            "<main><h1>Package Gate</h1></main>",
        ),
        ("client/manifest.json", Self::MANIFEST),
    ];
    const ICON: &'static str = "client/shared/icon-256.png";
    const DEB_ARCHITECTURES: [(&'static str, &'static str); 2] =
        [("x86_64", "amd64"), ("aarch64", "arm64")];

    fn deb_architecture() -> &'static str {
        Self::DEB_ARCHITECTURES
            .into_iter()
            .find(|(machine, _)| *machine == std::env::consts::ARCH)
            .map(|(_, architecture)| architecture)
            .expect("a packaged machine")
    }

    fn deb() -> String {
        format!("packagegate_0.0.1-7_{}.deb", Self::deb_architecture())
    }

    fn app_image() -> String {
        format!("PackageGate-0.0.1-7-{}.AppImage", std::env::consts::ARCH)
    }

    fn new() -> Self {
        let fixture = Self {
            workspace: tempfile::tempdir().expect("workspace"),
        };
        for (relative, contents) in Self::FILES {
            Self::write(&fixture.project().join(relative), contents.as_bytes());
        }
        Self::write(
            &fixture.project().join(Self::ICON),
            b"\x89PNG\r\n\x1a\nicon",
        );
        fixture
    }

    fn write(path: &Path, contents: &[u8]) {
        fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        fs::write(path, contents).expect("file");
    }

    fn project(&self) -> PathBuf {
        self.workspace.path().join("project")
    }

    async fn release(&self, output: &str) -> PathBuf {
        let output = self.workspace.path().join(output);
        let options = NativeBuildOptions {
            output_directory: output.clone(),
            target: NativeTarget::Linux,
            release: true,
            ..NativeBuildOptions::default()
        };
        let result = NativeCompiler::build_async(self.project(), &options)
            .await
            .unwrap_or_else(|failure| panic!("{failure}"));
        assert!(result.application_bundle().ends_with(Self::app_image()));
        output.join("linux")
    }

    async fn run(program: &str, arguments: &[&str], folder: &Path) -> String {
        super::host::run_tool_in(program, arguments, Some(folder))
            .await
            .unwrap_or_else(|failure| panic!("{failure}"))
    }

    async fn deb_listing(linux: &Path) -> String {
        Self::run("dpkg-deb", &["--contents", &Self::deb()], linux).await
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, dpkg-deb, mksquashfs and the AppImage runtime; run explicitly on the Linux lane"]
async fn a_linux_release_builds_the_same_app_image_and_deb_twice() {
    let fixture = LinuxPackageTests::new();
    let first = fixture.release("first").await;
    let second = fixture.release("second").await;
    for package in [LinuxPackageTests::deb(), LinuxPackageTests::app_image()] {
        assert_eq!(
            fs::read(first.join(&package)).expect("first"),
            fs::read(second.join(&package)).expect("second"),
            "{package} differs between two release builds"
        );
    }
    let manifest = fs::read_to_string(first.join("artifact-manifest.json")).expect("manifest");
    assert!(manifest.contains(&LinuxPackageTests::deb()), "{manifest}");
    assert!(
        manifest.contains(&LinuxPackageTests::app_image()),
        "{manifest}"
    );
    assert!(first.join("PackageGate/bin/PackageGate").is_file());
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, dpkg-deb and mksquashfs; run explicitly on the Linux lane"]
async fn the_deb_installs_the_app_its_command_desktop_file_metainfo_and_icon() {
    let fixture = LinuxPackageTests::new();
    let linux = fixture.release("dist").await;
    let fields = LinuxPackageTests::run(
        "dpkg-deb",
        &[
            "--field",
            &LinuxPackageTests::deb(),
            "Package",
            "Version",
            "Architecture",
            "Depends",
        ],
        &linux,
    )
    .await;
    let architecture = LinuxPackageTests::deb_architecture();
    assert_eq!(
        fields,
        format!(
            "Package: packagegate\nVersion: 0.0.1-7\nArchitecture: {architecture}\nDepends: libc6, libgtk-4-1, libwebkitgtk-6.0-4 (>= 2.40)\n"
        )
    );
    let listing = LinuxPackageTests::deb_listing(&linux).await;
    for entry in [
        "root/root",
        "./usr/bin/packagegate -> ../lib/packagegate/bin/PackageGate",
        "./usr/lib/packagegate/bin/PackageGate",
        "./usr/lib/packagegate/resources/NativeIndex.json",
        "./usr/share/applications/dev.tachyon.package_gate.desktop",
        "./usr/share/metainfo/dev.tachyon.package_gate.metainfo.xml",
        "./usr/share/icons/hicolor/256x256/apps/dev.tachyon.package_gate.png",
    ] {
        assert!(listing.contains(entry), "{entry} missing from\n{listing}");
    }
    assert!(!listing.contains("1000/1000"), "{listing}");
}

#[cfg(target_os = "linux")]
#[tokio::test]
#[ignore = "Requires GTK 4, WebKitGTK 6.0 development files, mksquashfs and the AppImage runtime; run explicitly on the Linux lane"]
async fn the_app_image_holds_an_app_dir_that_starts_the_app() {
    let fixture = LinuxPackageTests::new();
    let linux = fixture.release("dist").await;
    let extract = fixture.workspace.path().join("extract");
    fs::create_dir_all(&extract).expect("extract");
    let app_image = linux.join(LinuxPackageTests::app_image());
    let program = app_image.to_str().expect("path");
    LinuxPackageTests::run(program, &["--appimage-extract"], &extract).await;
    let root = extract.join("squashfs-root");
    assert_eq!(
        fs::read_link(root.join("AppRun")).expect("AppRun"),
        Path::new("usr/bin/packagegate")
    );
    assert!(
        root.join("AppRun")
            .canonicalize()
            .expect("target")
            .ends_with("usr/lib/packagegate/bin/PackageGate")
    );
    let desktop =
        fs::read_to_string(root.join("dev.tachyon.package_gate.desktop")).expect("desktop");
    assert!(
        desktop.contains("Exec=packagegate\nIcon=dev.tachyon.package_gate\n"),
        "{desktop}"
    );
    assert!(root.join("dev.tachyon.package_gate.png").is_file());
    assert!(root.join(".DirIcon").is_file());
    assert!(
        root.join("usr/share/metainfo/dev.tachyon.package_gate.metainfo.xml")
            .is_file()
    );
}

#[tokio::test]
async fn a_release_with_a_version_debian_cannot_hold_stops_before_the_build() {
    let fixture = LinuxPackageTests::new();
    let manifest = LinuxPackageTests::MANIFEST.replace("\"0.0.1\"", "\"v0.0.1\"");
    LinuxPackageTests::write(
        &fixture.project().join("client/manifest.json"),
        manifest.as_bytes(),
    );
    let options = NativeBuildOptions {
        output_directory: fixture.workspace.path().join("dist"),
        target: NativeTarget::Linux,
        release: true,
        ..NativeBuildOptions::default()
    };
    let text = NativeCompiler::build_async(fixture.project(), &options)
        .await
        .expect_err("refused")
        .to_string();
    assert!(text.contains("TY1601"), "{text}");
    assert!(
        text.contains("'v0.0.1' cannot be a Debian package version"),
        "{text}"
    );
    assert!(!fixture.workspace.path().join("dist/linux").exists());
}
