use super::MacOsArchitecture;
use crate::native::mac_os_slice::MacOsSlice;
use crate::native::windows_architecture::WindowsArchitecture;
use tachyon_contracts::NativeTarget;

struct MacOsArchitectureTests;

impl MacOsArchitectureTests {
    fn artifact_architecture(architecture: MacOsArchitecture) -> String {
        crate::native::compiler::artifact_target(
            NativeTarget::Macos,
            WindowsArchitecture::default(),
            architecture,
            crate::native::ios_destination::IosDestination::SIMULATOR,
        )
        .architecture
    }
}

#[test]
fn a_universal_app_is_built_from_an_arm64_and_an_x86_64_part() {
    let slices = MacOsArchitecture::UNIVERSAL.slices();
    assert_eq!(slices, [MacOsSlice::ARM64, MacOsSlice::X64]);
    let rust: Vec<&str> = slices.iter().map(|slice| slice.rust_target()).collect();
    assert_eq!(rust, ["aarch64-apple-darwin", "x86_64-apple-darwin"]);
    let swift: Vec<String> = slices
        .iter()
        .map(|slice| slice.swift_target("13.0"))
        .collect();
    assert_eq!(swift, ["arm64-apple-macosx13.0", "x86_64-apple-macosx13.0"]);
    assert!(!MacOsArchitecture::UNIVERSAL.is_build_machine());
}

#[test]
fn the_artifact_manifest_names_the_chosen_macos_architecture() {
    for (architecture, expected) in [
        (MacOsArchitecture::ARM64, "aarch64"),
        (MacOsArchitecture::X64, "x86_64"),
        (MacOsArchitecture::UNIVERSAL, "universal"),
    ] {
        assert_eq!(
            MacOsArchitectureTests::artifact_architecture(architecture),
            expected
        );
    }
}

#[test]
fn the_default_macos_architecture_is_the_build_machine() {
    let expected = if cfg!(target_arch = "aarch64") {
        MacOsArchitecture::ARM64
    } else {
        MacOsArchitecture::X64
    };
    assert_eq!(MacOsArchitecture::default(), expected);
    assert!(expected.is_build_machine());
    assert_eq!(expected.slices().len(), 1);
}
