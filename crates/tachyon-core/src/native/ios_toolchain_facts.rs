use super::host::{first_line, run_tool};
use crate::Failure;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct IosToolchainFacts {
    pub(super) xcode_version: String,
    pub(super) xcode_build: String,
    pub(super) sdk_version: String,
    pub(super) sdk_build: String,
    pub(super) machine_build: String,
}

impl IosToolchainFacts {
    const COMPILER: &'static str = "com.apple.compilers.llvm.clang.1_0";
    const XCODE_PREFIX: &'static str = "Xcode ";
    const BUILD_PREFIX: &'static str = "Build version ";

    pub(super) async fn query(sdk: &str) -> Result<Self, Failure> {
        let xcode = run_tool("/usr/bin/xcodebuild", &["-version"]).await?;
        let sdk_version = run_tool("/usr/bin/xcrun", &["--sdk", sdk, "--show-sdk-version"]).await?;
        let sdk_build = run_tool(
            "/usr/bin/xcrun",
            &["--sdk", sdk, "--show-sdk-build-version"],
        )
        .await?;
        let machine_build = run_tool("/usr/bin/sw_vers", &["-buildVersion"]).await?;
        Ok(Self::from_outputs(
            &xcode,
            &sdk_version,
            &sdk_build,
            &machine_build,
        ))
    }

    pub(super) fn from_outputs(
        xcode: &str,
        sdk_version: &str,
        sdk_build: &str,
        machine_build: &str,
    ) -> Self {
        Self {
            xcode_version: Self::after_prefix(xcode, Self::XCODE_PREFIX),
            xcode_build: Self::after_prefix(xcode, Self::BUILD_PREFIX),
            sdk_version: first_line(sdk_version, ""),
            sdk_build: first_line(sdk_build, ""),
            machine_build: first_line(machine_build, ""),
        }
    }

    pub(super) fn xcode_number(&self) -> String {
        let mut parts = self.xcode_version.split('.');
        let major: u32 = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        let minor: u32 = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        let patch: u32 = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
        format!("{major:02}{minor}{patch}")
    }

    pub(super) fn plist_keys(&self, platform: &str) -> String {
        format!(
            "\n<key>BuildMachineOSBuild</key><string>{machine}</string>\
             \n<key>DTCompiler</key><string>{compiler}</string>\
             \n<key>DTPlatformBuild</key><string>{sdk_build}</string>\
             \n<key>DTPlatformName</key><string>{platform}</string>\
             \n<key>DTPlatformVersion</key><string>{sdk_version}</string>\
             \n<key>DTSDKBuild</key><string>{sdk_build}</string>\
             \n<key>DTSDKName</key><string>{platform}{sdk_version}</string>\
             \n<key>DTXcode</key><string>{xcode}</string>\
             \n<key>DTXcodeBuild</key><string>{xcode_build}</string>",
            machine = super::host::xml_escape(&self.machine_build),
            compiler = Self::COMPILER,
            sdk_build = super::host::xml_escape(&self.sdk_build),
            sdk_version = super::host::xml_escape(&self.sdk_version),
            xcode = self.xcode_number(),
            xcode_build = super::host::xml_escape(&self.xcode_build),
        )
    }

    fn after_prefix(output: &str, prefix: &str) -> String {
        output
            .lines()
            .find_map(|line| line.trim().strip_prefix(prefix))
            .unwrap_or_default()
            .trim()
            .to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::IosToolchainFacts;

    #[test]
    fn xcode_and_sdk_versions_become_the_plist_build_keys() {
        let facts = IosToolchainFacts::from_outputs(
            "Xcode 16.2\nBuild version 16C5032a\n",
            "18.2\n",
            "22C146\n",
            "24C101\n",
        );
        assert_eq!(facts.xcode_version, "16.2");
        assert_eq!(facts.xcode_build, "16C5032a");
        assert_eq!(facts.xcode_number(), "1620");
        let keys = facts.plist_keys("iphoneos");
        assert!(keys.contains("<key>DTSDKName</key><string>iphoneos18.2</string>"));
        assert!(keys.contains("<key>DTPlatformName</key><string>iphoneos</string>"));
        assert!(keys.contains("<key>DTPlatformBuild</key><string>22C146</string>"));
        assert!(keys.contains("<key>DTXcode</key><string>1620</string>"));
        assert!(keys.contains("<key>DTXcodeBuild</key><string>16C5032a</string>"));
        assert!(keys.contains("<key>BuildMachineOSBuild</key><string>24C101</string>"));
    }

    #[test]
    fn the_xcode_number_has_four_digits() {
        let number = |version: &str| {
            IosToolchainFacts {
                xcode_version: String::from(version),
                ..IosToolchainFacts::default()
            }
            .xcode_number()
        };
        assert_eq!(number("26.0.1"), "2601");
        assert_eq!(number("26"), "2600");
        assert_eq!(number("9.4.1"), "0941");
    }
}
