use super::AndroidArchitecture;
use crate::native::MacOsArchitecture;
use crate::native::ios_destination::IosDestination;
use crate::native::windows_architecture::WindowsArchitecture;
use tachyon_contracts::NativeTarget;

struct AndroidArchitectureTests;

impl AndroidArchitectureTests {
    fn artifact_architecture(architecture: AndroidArchitecture) -> String {
        crate::native::compiler::artifact_target(
            NativeTarget::Android,
            WindowsArchitecture::default(),
            MacOsArchitecture::default(),
            architecture,
            IosDestination::SIMULATOR,
        )
        .architecture
    }
}

#[test]
fn an_android_build_holds_both_64_bit_abis_unless_one_is_chosen() {
    assert_eq!(
        AndroidArchitecture::default(),
        AndroidArchitecture::UNIVERSAL
    );
    assert_eq!(
        AndroidArchitecture::UNIVERSAL.abis(),
        [
            ("arm64-v8a", "aarch64-linux-android"),
            ("x86_64", "x86_64-linux-android"),
        ]
    );
    assert_eq!(
        AndroidArchitecture::ARM64.abis(),
        [("arm64-v8a", "aarch64-linux-android")]
    );
    assert_eq!(
        AndroidArchitecture::X64.abis(),
        [("x86_64", "x86_64-linux-android")]
    );
    assert_eq!(
        AndroidArchitecture::UNIVERSAL.gradle_abi_filters(),
        "\"arm64-v8a\", \"x86_64\""
    );
    assert_eq!(
        AndroidArchitecture::ARM64.gradle_abi_filters(),
        "\"arm64-v8a\""
    );
}

#[test]
fn the_artifact_manifest_names_the_chosen_android_architecture() {
    for (architecture, expected) in [
        (AndroidArchitecture::ARM64, "aarch64"),
        (AndroidArchitecture::X64, "x86_64"),
        (AndroidArchitecture::UNIVERSAL, "universal"),
    ] {
        assert_eq!(
            AndroidArchitectureTests::artifact_architecture(architecture),
            expected
        );
    }
    let machine = AndroidArchitecture::build_machine();
    assert_eq!(machine.abis().len(), 1);
    assert_ne!(machine.name(), "universal");
}
