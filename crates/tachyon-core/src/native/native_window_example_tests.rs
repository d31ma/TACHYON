#![allow(clippy::expect_used)]

use std::path::{Path, PathBuf};

struct NativeWindowExampleCheck;

impl NativeWindowExampleCheck {
    const SWIFT_CHECK: &str =
        include_str!("../../../../scripts/native/NativeWindowExampleCheck.swift");
    const CSHARP_CHECK: &str = include_str!("../../../../scripts/native/WindowExampleCheck.cs");
    const CSHARP_PROJECT: &str = "<Project Sdk=\"Microsoft.NET.Sdk\"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net9.0</TargetFramework><AllowUnsafeBlocks>true</AllowUnsafeBlocks><Nullable>disable</Nullable><RestoreIgnoreFailedSources>true</RestoreIgnoreFailedSources></PropertyGroup></Project>";

    async fn run(root: &Path) {
        Self::macos(&root.join("rs"), true).await;
        Self::macos(&root.join("swift"), false).await;
        Self::csharp(&root.join("cs")).await;
        Self::kotlin(&root.join("kt")).await;
    }

    async fn macos(project: &Path, rust: bool) {
        let checked = Self::macos_check(project, rust);
        let linked = Self::macos_companion(project, rust).await;
        let executable = project.join("window-check");
        Self::tool(
            "/usr/bin/xcrun",
            &[
                "swiftc",
                "-parse-as-library",
                "-framework",
                "AppKit",
                "-framework",
                "WebKit",
                &checked.to_string_lossy(),
                &linked.to_string_lossy(),
                "-o",
                &executable.to_string_lossy(),
            ],
            project,
        )
        .await;
        let output = Self::tool(&executable.to_string_lossy(), &[], project).await;
        println!("{}: {}", project.display(), output.trim());
    }

    fn macos_check(project: &Path, rust: bool) -> PathBuf {
        let host = Self::artifact(project, "TachyonHost.swift");
        let source = std::fs::read_to_string(host).expect("generated macOS host source");
        let end = source
            .rfind("@main")
            .expect("generated macOS host entry point");
        let (call, registration) = if rust {
            (
                "tacRustInvoke",
                "tacRustNativeSetCapabilityAdapter(tacRustCapabilityCall, tacRustCapabilityFree)",
            )
        } else {
            (
                "tacNativeInvoke",
                "NativeCapabilities.call = { capability, payload in MainActor.assumeIsolated { NativeBridge.shared.handle(capability, payload) } }",
            )
        };
        let check = Self::SWIFT_CHECK
            .replace("__CALL__", call)
            .replace("__REGISTER__", registration);
        let checked = project.join("WindowCheck.swift");
        std::fs::write(&checked, format!("{}{check}", &source[..end]))
            .expect("native window check");
        checked
    }

    async fn macos_companion(project: &Path, rust: bool) -> PathBuf {
        if !rust {
            return Self::artifact(project, "TachyonCompanion.swift");
        }
        let companion = Self::artifact(project, "companion.rs");
        let library = project.join("report.a");
        Self::tool(
            "rustc",
            &[
                "--edition",
                "2024",
                "--crate-type",
                "staticlib",
                "-O",
                &companion.to_string_lossy(),
                "-o",
                &library.to_string_lossy(),
            ],
            project,
        )
        .await;
        library
    }

    async fn csharp(project: &Path) {
        let companion = Self::artifact(project, "TachyonCompanion.cs");
        let check = project.join("managed-check");
        std::fs::create_dir_all(&check).expect("managed check directory");
        let source = std::fs::read_to_string(companion).expect("generated C# companion");
        std::fs::write(
            check.join("Program.cs"),
            format!("{source}{}", Self::CSHARP_CHECK),
        )
        .expect("managed check source");
        let manifest = check.join("check.csproj");
        std::fs::write(&manifest, Self::CSHARP_PROJECT).expect("managed project");
        let output = Self::tool(
            "dotnet",
            &["run", "--project", &manifest.to_string_lossy()],
            project,
        )
        .await;
        println!("{}", output.trim());
    }

    async fn kotlin(project: &Path) {
        let companion = Self::artifact(project, "TachyonCompanion.kt");
        let android = std::env::var_os("ANDROID_HOME")
            .or_else(|| std::env::var_os("ANDROID_SDK_ROOT"))
            .expect("Android SDK required for the qualified Kotlin check");
        let sdk = PathBuf::from(android).join("platforms/android-35/android.jar");
        assert!(
            sdk.is_file(),
            "Android API35 SDK required: {}",
            sdk.display()
        );
        Self::tool(
            "kotlinc",
            &[
                &companion.to_string_lossy(),
                "-classpath",
                &sdk.to_string_lossy(),
                "-d",
                &project.join("report.jar").to_string_lossy(),
            ],
            project,
        )
        .await;
        println!(
            "kt: complete generated ReportService companion typechecked against Android API35; no Android OS runtime claim"
        );
    }

    async fn tool(program: &str, arguments: &[&str], project: &Path) -> String {
        super::host::run_tool_in(program, arguments, Some(project))
            .await
            .expect("supervised native website example tool")
    }

    fn artifact(project: &Path, name: &str) -> PathBuf {
        Self::find(&project.join("dist"), name)
            .expect("ordinary CLI generated required native artifact")
    }

    fn find(directory: &Path, name: &str) -> Option<PathBuf> {
        let mut directories = vec![directory.to_path_buf()];
        while let Some(current) = directories.pop() {
            for entry in std::fs::read_dir(current).expect("generated artifact directory") {
                let entry = entry.expect("generated artifact entry");
                let kind = entry.file_type().expect("artifact type");
                if kind.is_file() && entry.file_name() == name {
                    return Some(entry.path());
                }
                if kind.is_dir() {
                    directories.push(entry.path());
                }
            }
        }
        None
    }
}

#[tokio::test]
#[ignore = "requires staged native website examples and macOS/.NET/Android SDKs"]
async fn staged_native_window_examples_use_supervised_tools() {
    let root =
        std::env::var_os("TAC_NATIVE_WINDOW_OUT").expect("explicit staged example root required");
    NativeWindowExampleCheck::run(&PathBuf::from(root)).await;
}
