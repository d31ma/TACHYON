use super::compiler::NativeBuildOptions;
use super::system_version::SystemVersion;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct IosDestination {
    pub(super) sdk: &'static str,
    pub(super) abi: &'static str,
    pub(super) signed: bool,
    pub(super) app_store: bool,
    platform: &'static str,
    platform_keys: &'static str,
    triple_suffix: &'static str,
    architecture: Option<&'static str>,
    artifact_architecture: Option<&'static str>,
    rust_target: Option<&'static str>,
}

impl IosDestination {
    pub(super) const SIMULATOR: Self = Self {
        sdk: "iphonesimulator",
        abi: "wkwebview-simulator",
        signed: false,
        app_store: false,
        platform: "iPhoneSimulator",
        platform_keys: "\n<key>DTPlatformName</key><string>iphonesimulator</string>",
        triple_suffix: "-simulator",
        architecture: None,
        artifact_architecture: None,
        rust_target: None,
    };
    pub(super) const DEVICE: Self = Self {
        sdk: "iphoneos",
        abi: "wkwebview",
        signed: true,
        app_store: false,
        platform: "iPhoneOS",
        platform_keys: "\n<key>LSRequiresIPhoneOS</key><true/>\
            \n<key>UIRequiredDeviceCapabilities</key><array><string>arm64</string></array>\
            \n<key>UIRequiresFullScreen</key><true/>",
        triple_suffix: "",
        architecture: Some("arm64"),
        artifact_architecture: Some("aarch64"),
        rust_target: Some("aarch64-apple-ios"),
    };
    pub(super) const APP_STORE: Self = Self {
        app_store: true,
        ..Self::DEVICE
    };
    const BY_CHOICE: [[Self; 2]; 2] = [
        [Self::SIMULATOR, Self::DEVICE],
        [Self::APP_STORE, Self::APP_STORE],
    ];
    const SIMULATOR_ARCHITECTURES: [&'static str; 2] = ["x86_64", "arm64"];
    const SIMULATOR_RUST_TARGETS: [&'static str; 2] = ["x86_64-apple-ios", "aarch64-apple-ios-sim"];

    pub(super) fn of(options: &NativeBuildOptions) -> Self {
        Self::BY_CHOICE[usize::from(options.release)][usize::from(options.device)]
    }

    pub(super) fn triple(self, deployment: SystemVersion) -> String {
        let simulator = Self::SIMULATOR_ARCHITECTURES[usize::from(cfg!(target_arch = "aarch64"))];
        format!(
            "{}-apple-ios{deployment}{}",
            self.architecture.unwrap_or(simulator),
            self.triple_suffix
        )
    }

    pub(super) fn rust_target(self) -> &'static str {
        let simulator = Self::SIMULATOR_RUST_TARGETS[usize::from(cfg!(target_arch = "aarch64"))];
        self.rust_target.unwrap_or(simulator)
    }

    pub(super) fn artifact_architecture(self, build_machine: &'static str) -> &'static str {
        self.artifact_architecture.unwrap_or(build_machine)
    }

    pub(super) fn plist_keys(self, toolchain: &str) -> String {
        format!(
            "\n<key>CFBundleSupportedPlatforms</key><array><string>{}</string></array>{}{toolchain}",
            self.platform, self.platform_keys
        )
    }
}

#[cfg(test)]
mod tests {
    use super::IosDestination;
    use crate::native::compiler::NativeBuildOptions;
    use crate::native::system_version::SystemVersion;

    fn options(device: bool, release: bool) -> NativeBuildOptions {
        NativeBuildOptions {
            release,
            device,
            ..NativeBuildOptions::default()
        }
    }

    #[test]
    fn device_and_release_choose_where_the_ios_app_runs() {
        assert_eq!(
            IosDestination::of(&options(false, false)),
            IosDestination::SIMULATOR
        );
        assert_eq!(
            IosDestination::of(&options(true, false)),
            IosDestination::DEVICE
        );
        assert_eq!(
            IosDestination::of(&options(false, true)),
            IosDestination::APP_STORE
        );
        assert_eq!(
            IosDestination::of(&options(true, true)),
            IosDestination::APP_STORE
        );
    }

    #[test]
    fn a_device_build_is_arm64_for_iphoneos() {
        let deployment = SystemVersion::new(18, 2);
        assert_eq!(
            IosDestination::DEVICE.triple(deployment),
            "arm64-apple-ios18.2"
        );
        assert!(
            IosDestination::SIMULATOR
                .triple(deployment)
                .ends_with("-apple-ios18.2-simulator")
        );
        assert_eq!(IosDestination::APP_STORE.sdk, "iphoneos");
        assert_eq!(
            IosDestination::DEVICE.artifact_architecture("x86_64"),
            "aarch64"
        );
        assert_eq!(
            IosDestination::SIMULATOR.artifact_architecture("x86_64"),
            "x86_64"
        );
    }

    #[test]
    fn rust_services_build_for_the_ios_system_the_app_runs_on() {
        assert_eq!(IosDestination::DEVICE.rust_target(), "aarch64-apple-ios");
        assert_eq!(IosDestination::APP_STORE.rust_target(), "aarch64-apple-ios");
        let simulator = ["x86_64-apple-ios", "aarch64-apple-ios-sim"];
        assert_eq!(
            IosDestination::SIMULATOR.rust_target(),
            simulator[usize::from(cfg!(target_arch = "aarch64"))]
        );
    }

    #[test]
    fn the_plist_names_the_platform_the_app_runs_on() {
        let simulator = IosDestination::SIMULATOR.plist_keys("");
        assert!(simulator.contains("<array><string>iPhoneSimulator</string></array>"));
        assert!(simulator.contains("<key>DTPlatformName</key><string>iphonesimulator</string>"));
        assert!(!simulator.contains("LSRequiresIPhoneOS"));
        let device = IosDestination::DEVICE.plist_keys("\n<key>DTXcode</key><string>1620</string>");
        assert!(device.contains("<array><string>iPhoneOS</string></array>"));
        assert!(device.contains("<key>LSRequiresIPhoneOS</key><true/>"));
        assert!(device.contains("<key>UIRequiresFullScreen</key><true/>"));
        assert!(device.ends_with("<key>DTXcode</key><string>1620</string>"));
    }
}
