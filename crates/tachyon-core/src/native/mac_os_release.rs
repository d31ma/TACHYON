use super::config::NativeApplication;
use super::host::{NativeHost, native_io, run_tool, write};
use super::mac_os_associated_domains::MacOsAssociatedDomains;
use super::mac_os_disk_image::MacOsDiskImage;
use super::mac_os_entitlements::MacOsEntitlements;
use super::mac_os_keychain_identity::MacOsKeychainIdentity;
use super::mac_os_notarization::MacOsNotarization;
use super::mac_os_signing::MacOsSigning;
use super::macos::unicode_path;
use crate::Failure;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) struct MacOsRelease<'build> {
    pub(super) signing: &'build MacOsSigning,
    pub(super) application: &'build NativeApplication,
    pub(super) stage: &'build Path,
    pub(super) bundle: &'build Path,
}

impl MacOsRelease<'_> {
    const TIMESTAMPS: [&'static str; 2] = ["--timestamp", "--timestamp=none"];
    const CODESIGN: &'static str = "/usr/bin/codesign";
    const XCRUN: &'static str = "/usr/bin/xcrun";
    const UNICODE: &'static str = "Release package path is not valid Unicode.";

    pub(super) async fn package(&self) -> Result<PathBuf, Failure> {
        let identity = self.signing_identity().await?;
        let domains =
            MacOsAssociatedDomains::prepare(self.signing, self.application, self.stage).await?;
        let entitlements =
            self.write_entitlements(MacOsAssociatedDomains::entries(domains.as_ref()))?;
        MacOsAssociatedDomains::embed(domains.as_ref(), self.bundle)?;
        self.sign_helpers(&identity).await?;
        self.sign_app(&identity, &entitlements).await?;
        self.verify_app().await?;
        let name = format!("{}.dmg", self.application.executable_name);
        let dmg = self.stage.join(&name);
        MacOsDiskImage::create(&self.application.name, self.bundle, &dmg).await?;
        self.distribute_dmg(&identity, &dmg).await?;
        Ok(PathBuf::from(name))
    }

    pub(super) fn codesign_arguments<'value>(
        identity: &'value str,
        ad_hoc: bool,
        entitlements: Option<&'value str>,
        target: &'value str,
    ) -> Vec<&'value str> {
        let mut arguments = vec![
            "--force",
            "--options",
            "runtime",
            Self::TIMESTAMPS[usize::from(ad_hoc)],
            "--sign",
            identity,
        ];
        arguments.extend(
            entitlements
                .into_iter()
                .flat_map(|path| ["--entitlements", path]),
        );
        arguments.push(target);
        arguments
    }

    async fn signing_identity(&self) -> Result<String, Failure> {
        if let Some(named) = self.signing.named_identity() {
            return Ok(named.to_owned());
        }
        let listing = run_tool("/usr/bin/security", &MacOsKeychainIdentity::FIND_ARGUMENTS).await?;
        MacOsKeychainIdentity::developer_id(&listing, self.signing.team())
    }

    fn write_entitlements(&self, signed_entries: &str) -> Result<PathBuf, Failure> {
        let path = self
            .stage
            .join("project")
            .join(MacOsEntitlements::FILE_NAME);
        let plist = MacOsEntitlements::plist(&self.application.permissions, signed_entries);
        write(&path, plist.as_bytes())?;
        Ok(path)
    }

    fn helpers(&self) -> Result<Vec<PathBuf>, Failure> {
        let folder = self.bundle.join("Contents").join("MacOS");
        let main = OsStr::new(&self.application.executable_name);
        let entries = native_io(fs::read_dir(&folder), &folder)?;
        let mut helpers: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file())
            .filter(|path| path.file_name() != Some(main))
            .collect();
        helpers.sort();
        Ok(helpers)
    }

    async fn sign_helpers(&self, identity: &str) -> Result<(), Failure> {
        for helper in self.helpers()? {
            let path = unicode_path(&helper, Self::UNICODE)?;
            let arguments =
                Self::codesign_arguments(identity, self.signing.is_ad_hoc(), None, path);
            run_tool(Self::CODESIGN, &arguments).await?;
        }
        Ok(())
    }

    async fn sign_app(&self, identity: &str, entitlements: &Path) -> Result<(), Failure> {
        let app = unicode_path(self.bundle, Self::UNICODE)?;
        let entitlements = unicode_path(entitlements, Self::UNICODE)?;
        let arguments =
            Self::codesign_arguments(identity, self.signing.is_ad_hoc(), Some(entitlements), app);
        run_tool(Self::CODESIGN, &arguments).await.map(|_| ())
    }

    async fn verify_app(&self) -> Result<(), Failure> {
        let app = unicode_path(self.bundle, Self::UNICODE)?;
        let arguments = ["--verify", "--strict", "--deep", "--verbose=2", app];
        run_tool(Self::CODESIGN, &arguments).await.map(|_| ())
    }

    async fn distribute_dmg(&self, identity: &str, dmg: &Path) -> Result<(), Failure> {
        if self.signing.is_ad_hoc() {
            return Ok(());
        }
        let image = unicode_path(dmg, Self::UNICODE)?;
        let profile = self.signing.notary_profile();
        let signing = ["--force", "--timestamp", "--sign", identity, image];
        run_tool(Self::CODESIGN, &signing).await?;
        let submission = MacOsNotarization::submit_arguments(image, profile);
        let answer = NativeHost::run_long_tool(Self::XCRUN, &submission).await?;
        MacOsNotarization::accepted(&answer, profile)?;
        run_tool(Self::XCRUN, &MacOsNotarization::staple_arguments(image)).await?;
        run_tool(Self::XCRUN, &MacOsNotarization::validate_arguments(image)).await?;
        run_tool(
            "/usr/sbin/spctl",
            &MacOsNotarization::gatekeeper_arguments(image),
        )
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MacOsRelease;

    #[test]
    fn a_developer_id_signature_uses_the_hardened_runtime_and_a_secure_timestamp() {
        assert_eq!(
            MacOsRelease::codesign_arguments(
                "2222222222222222222222222222222222222222",
                false,
                Some("project/TachyonHost.entitlements"),
                "Notes.app"
            ),
            [
                "--force",
                "--options",
                "runtime",
                "--timestamp",
                "--sign",
                "2222222222222222222222222222222222222222",
                "--entitlements",
                "project/TachyonHost.entitlements",
                "Notes.app"
            ]
        );
    }

    #[test]
    fn an_ad_hoc_signature_keeps_the_hardened_runtime_without_a_timestamp() {
        assert_eq!(
            MacOsRelease::codesign_arguments("-", true, None, "Notes.app/Contents/MacOS/helper"),
            [
                "--force",
                "--options",
                "runtime",
                "--timestamp=none",
                "--sign",
                "-",
                "Notes.app/Contents/MacOS/helper"
            ]
        );
    }
}
