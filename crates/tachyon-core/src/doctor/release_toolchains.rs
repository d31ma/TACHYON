use super::target_probe::TargetProbe;
use super::target_toolchain::TargetToolchain;
use crate::AppTarget;

pub(super) struct ReleaseToolchains;

impl ReleaseToolchains {
    pub(super) const ROWS: [TargetToolchain; 10] = [
        TargetToolchain {
            target: AppTarget::Macos,
            command: "xcrun notarytool",
            probe: TargetProbe::Version("xcrun", &["notarytool", "--version"]),
            install: "notarytool comes with Xcode 13 or newer. Store a notary profile with `xcrun notarytool store-credentials` and set TAC_APPLE_NOTARY_PROFILE to its name.",
        },
        TargetToolchain {
            target: AppTarget::Macos,
            command: "Developer ID Application",
            probe: TargetProbe::Identity(&["Developer ID Application"]),
            install: "Create a Developer ID Application certificate for your team (Xcode > Settings > Accounts > Manage Certificates) and keep it with its private key in the keychain. Set TAC_APPLE_TEAM_ID to the team.",
        },
        TargetToolchain {
            target: AppTarget::Ios,
            command: "xcrun --sdk iphoneos",
            probe: TargetProbe::Version("xcrun", &["--sdk", "iphoneos", "--show-sdk-version"]),
            install: "Open Xcode, go to Settings > Components, and install the iOS platform. Device and App Store builds use its iPhoneOS SDK.",
        },
        TargetToolchain {
            target: AppTarget::Ios,
            command: "Apple Distribution",
            probe: TargetProbe::Identity(&["Apple Distribution", "iPhone Distribution"]),
            install: "Create an Apple Distribution certificate for your team (Xcode > Settings > Accounts > Manage Certificates) and keep it with its private key in the keychain. Set TAC_IOS_PROVISIONING_PROFILE to an App Store profile that names it. A --device build uses an Apple Development certificate instead.",
        },
        TargetToolchain {
            target: AppTarget::Android,
            command: "apksigner",
            probe: TargetProbe::BuildTool {
                variables: &["ANDROID_HOME", "ANDROID_SDK_ROOT"],
                files: &["apksigner", "apksigner.bat"],
            },
            install: "Install the Android SDK Build-Tools in the SDK Manager, for example: sdkmanager \"build-tools;35.0.0\". They hold apksigner and zipalign, which sign and align a release APK.",
        },
        TargetToolchain {
            target: AppTarget::Android,
            command: "zipalign",
            probe: TargetProbe::BuildTool {
                variables: &["ANDROID_HOME", "ANDROID_SDK_ROOT"],
                files: &["zipalign", "zipalign.exe"],
            },
            install: "Install the Android SDK Build-Tools in the SDK Manager, for example: sdkmanager \"build-tools;35.0.0\". They hold apksigner and zipalign, which sign and align a release APK.",
        },
        TargetToolchain {
            target: AppTarget::Windows,
            command: "makeappx.exe",
            probe: TargetProbe::OnPath("makeappx.exe"),
            install: "makeappx.exe comes with the Windows SDK in the \"Desktop development with C++\" workload. Run ty from a Visual Studio developer prompt so that it is on PATH. It packs the MSIX package of a release.",
        },
        TargetToolchain {
            target: AppTarget::Windows,
            command: "signtool.exe",
            probe: TargetProbe::OnPath("signtool.exe"),
            install: "signtool.exe comes with the Windows SDK in the \"Desktop development with C++\" workload. Run ty from a Visual Studio developer prompt so that it is on PATH. It signs the MSIX package of a release.",
        },
        TargetToolchain {
            target: AppTarget::Linux,
            command: "dpkg-deb",
            probe: TargetProbe::Version("dpkg-deb", &["--version"]),
            install: "Install dpkg, which builds the .deb package of a release, for example: sudo apt-get install -y dpkg (sudo dnf install dpkg on Fedora, sudo pacman -S dpkg on Arch).",
        },
        TargetToolchain {
            target: AppTarget::Linux,
            command: "mksquashfs",
            probe: TargetProbe::Version("mksquashfs", &["-version"]),
            install: "Install squashfs-tools, which builds the AppImage of a release, for example: sudo apt-get install -y squashfs-tools (the package has the same name on Fedora and Arch).",
        },
    ];
}
