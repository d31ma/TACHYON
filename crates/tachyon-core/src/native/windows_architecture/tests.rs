use super::WindowsArchitecture;
use crate::native::windows::WindowsHostGenerator;
use crate::native::windows_compiler_plan::WindowsCompilerPlan;
use std::path::Path;

struct WindowsArchitectureTests;

impl WindowsArchitectureTests {
    const SDK: &'static str = r"C:\deps\webview2";
    const SOURCE: &'static str = r"C:\stage\project\tachyon_host.c";
    const EXECUTABLE: &'static str = r"C:\stage\Monitor\bin\Monitor.exe";

    fn arguments(architecture: WindowsArchitecture) -> Vec<String> {
        WindowsCompilerPlan {
            sdk: Path::new(Self::SDK),
            source: Self::SOURCE,
            executable: Self::EXECUTABLE,
            resource: None,
            architecture,
        }
        .arguments(WindowsHostGenerator::LINKER_ARGUMENTS)
    }

    fn library_path(architecture: WindowsArchitecture) -> String {
        Self::arguments(architecture)
            .into_iter()
            .find(|argument| argument.starts_with("/LIBPATH:"))
            .unwrap_or_default()
            .replace('\\', "/")
    }
}

#[test]
fn the_x64_build_plan_links_the_x64_webview2_loader_for_an_x64_machine() {
    let arguments = WindowsArchitectureTests::arguments(WindowsArchitecture::X64);
    assert!(arguments.contains(&String::from("/MACHINE:X64")));
    assert!(
        WindowsArchitectureTests::library_path(WindowsArchitecture::X64)
            .ends_with("/build/native/x64")
    );
    assert_eq!(
        WindowsArchitecture::X64.rust_target(),
        "x86_64-pc-windows-msvc"
    );
    assert!(
        WindowsHostGenerator::companion_project(WindowsArchitecture::X64)
            .contains("<RuntimeIdentifier>win-x64</RuntimeIdentifier>")
    );
    assert!(
        WindowsHostGenerator::published_companion(Path::new("companion"), WindowsArchitecture::X64)
            .ends_with("bin/Release/net9.0/win-x64/publish/TachyonCompanion.dll")
    );
    assert_eq!(WindowsArchitecture::X64.artifact_architecture(), "x86_64");
}

#[test]
fn the_arm64_build_plan_uses_arm64_for_every_architecture_specific_value() {
    let arguments = WindowsArchitectureTests::arguments(WindowsArchitecture::ARM64);
    assert!(arguments.contains(&String::from("/MACHINE:ARM64")));
    assert!(!arguments.iter().any(|argument| argument.contains("x64")));
    assert!(
        WindowsArchitectureTests::library_path(WindowsArchitecture::ARM64)
            .ends_with("/build/native/arm64")
    );
    assert_eq!(
        WindowsArchitecture::ARM64.rust_target(),
        "aarch64-pc-windows-msvc"
    );
    let project = WindowsHostGenerator::companion_project(WindowsArchitecture::ARM64);
    assert!(project.contains("<RuntimeIdentifier>win-arm64</RuntimeIdentifier>"));
    assert!(!project.contains("x64"));
    assert!(
        WindowsHostGenerator::published_companion(
            Path::new("companion"),
            WindowsArchitecture::ARM64
        )
        .ends_with("bin/Release/net9.0/win-arm64/publish/TachyonCompanion.dll")
    );
}

#[test]
fn the_build_plan_keeps_the_compiler_options_and_the_application_entry() {
    let arguments = WindowsArchitectureTests::arguments(WindowsArchitecture::ARM64);
    for expected in [
        "/W4",
        "/WX",
        "/std:c17",
        "/SUBSYSTEM:WINDOWS",
        "/ENTRY:wWinMainCRTStartup",
        "WebView2LoaderStatic.lib",
    ] {
        assert!(arguments.contains(&String::from(expected)), "{expected}");
    }
    let link = arguments
        .iter()
        .position(|argument| argument == "/link")
        .unwrap_or_default();
    let machine = arguments
        .iter()
        .position(|argument| argument == "/MACHINE:ARM64")
        .unwrap_or_default();
    assert!(link > 0 && machine > link);
    assert!(arguments.contains(&format!("/Fe:{}", WindowsArchitectureTests::EXECUTABLE)));
}

#[test]
fn the_default_architecture_is_the_build_machine_and_the_artifact_names_it() {
    let expected = if cfg!(target_arch = "aarch64") {
        WindowsArchitecture::ARM64
    } else {
        WindowsArchitecture::X64
    };
    assert_eq!(WindowsArchitecture::default(), expected);
    let target = crate::native::compiler::artifact_target(
        tachyon_contracts::NativeTarget::Windows,
        WindowsArchitecture::ARM64,
    );
    assert_eq!(target.architecture, "aarch64");
    assert_eq!(target.abi, "webview2");
}
