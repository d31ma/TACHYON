use super::compiler::NativeBuildOptions;
use super::config::NativeApplication;
use super::host::{native_io, native_tool_failure, run_tool, write};
use super::ios_app_icon::IosAppIcon;
use super::ios_destination::IosDestination;
use super::ios_entitlements::IosEntitlements;
use super::ios_provisioning_profile::IosProvisioningProfile;
use super::ios_signing::IosSigning;
use super::ios_signing_identity::IosSigningIdentity;
use super::ios_toolchain_facts::IosToolchainFacts;
use crate::Failure;
use std::path::{Path, PathBuf};
use tachyon_contracts::NativeTarget;

#[derive(Debug)]
pub(super) struct IosDeviceBuild {
    destination: IosDestination,
    signing: IosSigning,
    profile: IosProvisioningProfile,
    identity: String,
    facts: IosToolchainFacts,
    scratch: tempfile::TempDir,
}

impl IosDeviceBuild {
    const DECODED_PROFILE: &'static str = "profile.plist";
    const ENTITLEMENTS: &'static str = "entitlements.plist";
    const EMBEDDED_PROFILE: &'static str = "embedded.mobileprovision";
    const PAYLOAD: &'static str = "Payload";
    pub(super) const PRIVACY_MANIFEST_NAME: &'static str = "PrivacyInfo.xcprivacy";
    pub(super) const PRIVACY_MANIFEST: &'static str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>NSPrivacyTracking</key><false/>
<key>NSPrivacyTrackingDomains</key><array/>
<key>NSPrivacyCollectedDataTypes</key><array/>
<key>NSPrivacyAccessedAPITypes</key><array>
<dict><key>NSPrivacyAccessedAPIType</key><string>NSPrivacyAccessedAPICategoryFileTimestamp</string>
<key>NSPrivacyAccessedAPITypeReasons</key><array><string>C617.1</string></array></dict>
<dict><key>NSPrivacyAccessedAPIType</key><string>NSPrivacyAccessedAPICategoryUserDefaults</string>
<key>NSPrivacyAccessedAPITypeReasons</key><array><string>CA92.1</string></array></dict>
</array>
</dict></plist>
"#;

    pub(super) fn preflight(options: &NativeBuildOptions) -> Result<(), Failure> {
        Self::check_environment(options.target, IosDestination::of(options))
    }

    fn check_environment(target: NativeTarget, destination: IosDestination) -> Result<(), Failure> {
        if target != NativeTarget::Ios || !destination.signed {
            return Ok(());
        }
        IosSigning::from_process_environment().map(|_| ())
    }

    pub(super) async fn prepare(
        application: &NativeApplication,
        destination: IosDestination,
    ) -> Result<Option<Self>, Failure> {
        Self::prepare_when_signed(application, destination).await
    }

    async fn prepare_when_signed(
        application: &NativeApplication,
        destination: IosDestination,
    ) -> Result<Option<Self>, Failure> {
        if !destination.signed {
            return Ok(None);
        }
        Self::prepared(application, destination).await.map(Some)
    }

    pub(super) async fn prepared(
        application: &NativeApplication,
        destination: IosDestination,
    ) -> Result<Self, Failure> {
        Self::require_apple_machine()?;
        let signing = IosSigning::from_process_environment()?;
        let scratch = native_io(tempfile::tempdir(), Path::new("."))?;
        let decoded = scratch.path().join(Self::DECODED_PROFILE);
        run_tool(
            "/usr/bin/security",
            &[
                "cms",
                "-D",
                "-i",
                &signing.profile.to_string_lossy(),
                "-o",
                &decoded.to_string_lossy(),
            ],
        )
        .await?;
        let profile = IosProvisioningProfile::from_plist(native_io(
            std::fs::read_to_string(&decoded),
            &decoded,
        )?);
        profile.check(&signing, &application.application_id, destination.app_store)?;
        profile.require_associated_domains(!application.links.hosts().is_empty())?;
        let identities = run_tool(
            "/usr/bin/security",
            &["find-identity", "-v", "-p", "codesigning"],
        )
        .await?;
        let identity =
            IosSigningIdentity::choose(&identities, &profile, signing.identity.as_deref())?;
        let facts = IosToolchainFacts::query(destination.sdk).await?;
        Ok(Self {
            destination,
            signing,
            profile,
            identity,
            facts,
            scratch,
        })
    }

    pub(super) async fn finish(
        build: Option<Self>,
        application: &NativeApplication,
        web_bundle: &Path,
        stage: &Path,
        bundle: &Path,
    ) -> Result<PathBuf, Failure> {
        Self::finish_for_destination(build, application, web_bundle, stage, bundle).await
    }

    async fn finish_for_destination(
        build: Option<Self>,
        application: &NativeApplication,
        web_bundle: &Path,
        stage: &Path,
        bundle: &Path,
    ) -> Result<PathBuf, Failure> {
        let Some(build) = build else {
            IosEntitlements::sign(bundle).await?;
            return Ok(PathBuf::from(format!(
                "{}.app",
                application.executable_name
            )));
        };
        build.export(application, web_bundle, stage, bundle).await
    }

    pub(super) async fn export(
        self,
        application: &NativeApplication,
        web_bundle: &Path,
        stage: &Path,
        bundle: &Path,
    ) -> Result<PathBuf, Failure> {
        let deployment = application.minimum_system.floor(NativeTarget::Ios);
        let icon = IosAppIcon::source(application, web_bundle);
        IosAppIcon::require_for_store(icon.as_deref(), self.destination.app_store)?;
        let icon_keys = IosAppIcon::plist_keys(
            (icon.as_deref(), &application.window),
            bundle,
            self.scratch.path(),
            deployment,
            self.destination.app_store,
        )
        .await?;
        let toolchain = self.facts.plist_keys(self.destination.sdk);
        let keys = self
            .destination
            .plist_keys(&format!("{toolchain}{icon_keys}"));
        write(
            &bundle.join("Info.plist"),
            super::ios::info_plist(application, &[], &keys).as_bytes(),
        )?;
        write(
            &bundle.join(Self::PRIVACY_MANIFEST_NAME),
            Self::PRIVACY_MANIFEST.as_bytes(),
        )?;
        let embedded = bundle.join(Self::EMBEDDED_PROFILE);
        native_io(std::fs::copy(&self.signing.profile, &embedded), &embedded)?;
        self.sign(application, bundle).await?;
        self.write_ipa(application, stage, bundle).await
    }

    pub(super) async fn sign(
        &self,
        application: &NativeApplication,
        bundle: &Path,
    ) -> Result<(), Failure> {
        let entitlements = self.scratch.path().join(Self::ENTITLEMENTS);
        let text = self.profile.entitlements(
            &self.signing.team,
            &application.application_id,
            &IosEntitlements::associated_domains(application),
        );
        write(&entitlements, text.as_bytes())?;
        let bundle_text = bundle.to_string_lossy();
        run_tool(
            "/usr/bin/codesign",
            &[
                "--force",
                "--sign",
                &self.identity,
                "--entitlements",
                &entitlements.to_string_lossy(),
                "--generate-entitlement-der",
                "--timestamp=none",
                &bundle_text,
            ],
        )
        .await?;
        run_tool(
            "/usr/bin/codesign",
            &["--verify", "--strict", "--verbose=2", &bundle_text],
        )
        .await
        .map(|_| ())
    }

    pub(super) async fn write_ipa(
        &self,
        application: &NativeApplication,
        stage: &Path,
        bundle: &Path,
    ) -> Result<PathBuf, Failure> {
        let payload = self.scratch.path().join(Self::PAYLOAD);
        let copied = payload.join(format!("{}.app", application.executable_name));
        run_tool(
            "/usr/bin/ditto",
            &[&bundle.to_string_lossy(), &copied.to_string_lossy()],
        )
        .await?;
        let name = format!("{}.ipa", application.executable_name);
        let archive = stage.join(&name);
        run_tool(
            "/usr/bin/ditto",
            &[
                "-c",
                "-k",
                "--norsrc",
                "--noextattr",
                "--noacl",
                "--keepParent",
                &payload.to_string_lossy(),
                &archive.to_string_lossy(),
            ],
        )
        .await?;
        Ok(PathBuf::from(name))
    }

    fn require_apple_machine() -> Result<(), Failure> {
        cfg!(target_os = "macos").then_some(()).ok_or_else(|| {
            native_tool_failure(
                1605,
                "A device or App Store iOS build is signed with the keychain and Xcode, which only an Apple macOS build machine has.",
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::IosDeviceBuild;
    use crate::native::compiler::NativeBuildOptions;
    use tachyon_contracts::NativeTarget;

    #[test]
    fn only_a_signed_ios_build_reads_the_signing_environment() {
        let simulator = NativeBuildOptions {
            target: NativeTarget::Ios,
            ..NativeBuildOptions::default()
        };
        assert!(IosDeviceBuild::preflight(&simulator).is_ok());
        let android_release = NativeBuildOptions {
            target: NativeTarget::Android,
            release: true,
            device: true,
            ..NativeBuildOptions::default()
        };
        assert!(IosDeviceBuild::preflight(&android_release).is_ok());
    }

    #[test]
    fn the_privacy_manifest_declares_the_host_file_and_defaults_reads() {
        let manifest = IosDeviceBuild::PRIVACY_MANIFEST;
        assert!(manifest.contains("<key>NSPrivacyTracking</key><false/>"));
        assert!(manifest.contains("NSPrivacyAccessedAPICategoryFileTimestamp"));
        assert!(manifest.contains("<string>C617.1</string>"));
        assert!(manifest.contains("NSPrivacyAccessedAPICategoryUserDefaults"));
        assert!(manifest.contains("<string>CA92.1</string>"));
    }
}
