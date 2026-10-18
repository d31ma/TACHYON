use super::Toolchain;
use super::target_probe::TargetProbe;
use super::target_toolchain::TargetToolchain;
use crate::AppTarget;
use std::path::Path;

pub(crate) struct TargetToolchains;

impl TargetToolchains {
    const ROWS: [TargetToolchain; 12] = [
        TargetToolchain {
            target: AppTarget::Macos,
            command: "xcrun",
            probe: TargetProbe::Version("xcrun", &["swiftc", "--version"]),
            install: "Install Xcode, or only its command-line tools with `xcode-select --install`. macOS apps build only on a Mac.",
        },
        TargetToolchain {
            target: AppTarget::Ios,
            command: "xcodebuild",
            probe: TargetProbe::Version("xcodebuild", &["-version"]),
            install: "Install Xcode from the App Store (https://apps.apple.com/app/xcode/id497799835) and open it once to accept the license. iOS apps build only on a Mac.",
        },
        TargetToolchain {
            target: AppTarget::Ios,
            command: "xcrun --sdk iphonesimulator",
            probe: TargetProbe::Version(
                "xcrun",
                &["--sdk", "iphonesimulator", "--show-sdk-version"],
            ),
            install: "Open Xcode, go to Settings > Components, and install the iOS platform. It holds the simulator SDK that iOS apps build against.",
        },
        TargetToolchain {
            target: AppTarget::Android,
            command: "ANDROID_HOME",
            probe: TargetProbe::Folder {
                variables: &["ANDROID_HOME", "ANDROID_SDK_ROOT"],
                marker: "platforms",
            },
            install: "Install Android Studio or its command-line tools from https://developer.android.com/studio, install one Android SDK platform in the SDK Manager, and set ANDROID_HOME to the SDK folder.",
        },
        TargetToolchain {
            target: AppTarget::Android,
            command: "gradle",
            probe: TargetProbe::Version("gradle", &["--version"]),
            install: "Run `ty doctor --install`, which installs Gradle 8.14.5, or put Gradle 8.14 or newer on PATH.",
        },
        TargetToolchain {
            target: AppTarget::Android,
            command: "javac",
            probe: TargetProbe::Version("javac", &["-version"]),
            install: "Run `ty doctor --install`, which installs the Temurin 21 JDK, or put a JDK 17 or newer on PATH. The Android Gradle plugin needs JDK 17 or newer.",
        },
        TargetToolchain {
            target: AppTarget::Windows,
            command: "cl",
            probe: TargetProbe::OnPath("cl"),
            install: "Install the Visual Studio Build Tools with the \"Desktop development with C++\" workload, then run ty from a Visual Studio developer prompt so that cl is on PATH. Windows apps build only on Windows.",
        },
        TargetToolchain {
            target: AppTarget::Windows,
            command: "rc.exe",
            probe: TargetProbe::OnPath("rc.exe"),
            install: "rc.exe comes with the Windows SDK in the \"Desktop development with C++\" workload. Run ty from a Visual Studio developer prompt so that rc.exe is on PATH.",
        },
        TargetToolchain {
            target: AppTarget::Windows,
            command: "TAC_WEBVIEW2_SDK",
            probe: TargetProbe::Folder {
                variables: &["TAC_WEBVIEW2_SDK"],
                marker: "build/native/include/WebView2.h",
            },
            install: "Download the Microsoft.Web.WebView2 package from https://www.nuget.org/packages/Microsoft.Web.WebView2, unzip the .nupkg file, and set TAC_WEBVIEW2_SDK to that folder.",
        },
        TargetToolchain {
            target: AppTarget::Linux,
            command: "cc",
            probe: TargetProbe::Version("cc", &["--version"]),
            install: "Install a C compiler, for example: sudo apt-get install -y build-essential. Linux apps build only on Linux.",
        },
        TargetToolchain {
            target: AppTarget::Linux,
            command: "pkg-config gtk4",
            probe: TargetProbe::Version("pkg-config", &["--modversion", "gtk4"]),
            install: "Install pkg-config and GTK 4, for example: sudo apt-get install -y pkg-config libgtk-4-dev. Run `ty doctor --install` for Fedora and Arch.",
        },
        TargetToolchain {
            target: AppTarget::Linux,
            command: "pkg-config webkitgtk-6.0",
            probe: TargetProbe::Version("pkg-config", &["--modversion", "webkitgtk-6.0"]),
            install: "Install WebKitGTK 6.0, for example: sudo apt-get install -y libwebkitgtk-6.0-dev. Run `ty doctor --install` for Fedora and Arch.",
        },
    ];

    pub(crate) fn for_targets(targets: &[AppTarget]) -> Vec<Toolchain> {
        targets
            .iter()
            .flat_map(|target| Self::ROWS.iter().filter(|row| row.is_for(*target)))
            .map(TargetToolchain::toolchain)
            .collect()
    }

    pub(super) fn for_project(root: &Path) -> Vec<Toolchain> {
        Self::for_targets(&crate::scaffold::ProjectState::bundle_targets(root))
    }
}
