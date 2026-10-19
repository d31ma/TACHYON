use super::android_signing_certificate::AndroidSigningCertificate;
use super::apple_team::AppleTeam;
use super::config::NativeApplication;
use super::windows_package_family::WindowsPackageFamily;
use crate::Failure;
use serde_json::json;
use std::path::{Path, PathBuf};

pub(crate) struct SiteAssociation;

impl SiteAssociation {
    const APPLE_PATH: &'static str = ".well-known/apple-app-site-association";
    const ANDROID_PATH: &'static str = ".well-known/assetlinks.json";
    const WINDOWS_PATH: &'static str = ".well-known/windows-app-web-link";
    const APPLE_NOTE: &'static str = "Note: tac.application.links lists hosts, but TAC_APPLE_TEAM_ID is not set, so the build writes no /.well-known/apple-app-site-association. Until it is set, iOS opens the app only from its custom schemes.";
    const ANDROID_NOTE: &'static str = "Note: tac.application.links lists hosts, but the build writes /.well-known/assetlinks.json only when tac.origin and TAC_ANDROID_CERT_SHA256 are both set. Until then, Android does not verify the hosts and asks which app opens their links.";
    const WINDOWS_NOTE: &'static str = "Note: tac.application.links lists hosts, but TAC_WINDOWS_CERTIFICATE_PUBLISHER is not set, so the build writes no /.well-known/windows-app-web-link. Until it is set, Windows opens links to the hosts in the browser.";

    /// Returns the files that let a site hand its links to the native apps,
    /// and adds a note for each file it cannot write.
    ///
    /// # Errors
    ///
    /// Returns a failure when the manifest, `TAC_APPLE_TEAM_ID`,
    /// `TAC_ANDROID_CERT_SHA256` or `TAC_WINDOWS_CERTIFICATE_PUBLISHER` cannot
    /// be read.
    pub(crate) fn files(
        snapshot_root: &Path,
        authored_root: &Path,
        origin: Option<&str>,
        notes: &mut Vec<String>,
    ) -> Result<Vec<(PathBuf, Vec<u8>)>, Failure> {
        let application = NativeApplication::discover_from_snapshot(snapshot_root, authored_root)?;
        let team = AppleTeam::from_environment()?;
        let certificate = AndroidSigningCertificate::from_environment()?;
        let apple = Self::apple(&application, team.as_ref());
        let android = Self::android(&application, origin, &certificate);
        let family = WindowsPackageFamily::from_environment(&application.application_id)?;
        let windows = Self::windows(&application, family.as_ref());
        notes.extend(
            [apple.1, android.1, windows.1]
                .into_iter()
                .flatten()
                .map(String::from),
        );
        Ok([apple.0, android.0, windows.0]
            .into_iter()
            .flatten()
            .collect())
    }

    fn apple(
        application: &NativeApplication,
        team: Option<&AppleTeam>,
    ) -> (Option<(PathBuf, Vec<u8>)>, Option<&'static str>) {
        match (application.links.hosts().is_empty(), team) {
            (true, _) => (None, None),
            (false, None) => (None, Some(Self::APPLE_NOTE)),
            (false, Some(team)) => (Some(Self::apple_file(application, team)), None),
        }
    }

    fn android(
        application: &NativeApplication,
        origin: Option<&str>,
        certificate: &AndroidSigningCertificate,
    ) -> (Option<(PathBuf, Vec<u8>)>, Option<&'static str>) {
        let ready = [origin.is_some(), !certificate.fingerprints().is_empty()] == [true, true];
        match (application.links.hosts().is_empty(), ready) {
            (true, _) => (None, None),
            (false, false) => (None, Some(Self::ANDROID_NOTE)),
            (false, true) => (Some(Self::android_file(application, certificate)), None),
        }
    }

    fn windows(
        application: &NativeApplication,
        family: Option<&WindowsPackageFamily>,
    ) -> (Option<(PathBuf, Vec<u8>)>, Option<&'static str>) {
        let listed = !application.links.hosts().is_empty();
        let file = family.filter(|_| listed).map(Self::windows_file);
        let unsigned = [listed, family.is_none()] == [true, true];
        (file, unsigned.then_some(Self::WINDOWS_NOTE))
    }

    fn apple_file(application: &NativeApplication, team: &AppleTeam) -> (PathBuf, Vec<u8>) {
        let association = json!({
            "applinks": {
                "details": [{
                    "appIDs": [team.app_id(&application.application_id)],
                    "components": [{"/": "*"}]
                }]
            }
        });
        (PathBuf::from(Self::APPLE_PATH), Self::pretty(&association))
    }

    fn android_file(
        application: &NativeApplication,
        certificate: &AndroidSigningCertificate,
    ) -> (PathBuf, Vec<u8>) {
        let statements = json!([{
            "relation": ["delegate_permission/common.handle_all_urls"],
            "target": {
                "namespace": "android_app",
                "package_name": super::android::java_package(&application.application_id),
                "sha256_cert_fingerprints": certificate.fingerprints()
            }
        }]);
        (PathBuf::from(Self::ANDROID_PATH), Self::pretty(&statements))
    }

    fn windows_file(family: &WindowsPackageFamily) -> (PathBuf, Vec<u8>) {
        let link = json!([{
            "packageFamilyName": family.name(),
            "paths": ["*"]
        }]);
        (PathBuf::from(Self::WINDOWS_PATH), Self::pretty(&link))
    }

    fn pretty(value: &serde_json::Value) -> Vec<u8> {
        let mut bytes = serde_json::to_vec_pretty(value).unwrap_or_default();
        bytes.push(b'\n');
        bytes
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::SiteAssociation;
    use crate::native::android_signing_certificate::AndroidSigningCertificate;
    use crate::native::apple_team::AppleTeam;
    use crate::native::config::NativeApplication;
    use crate::native::native_links::NativeLinks;
    use crate::native::windows_package_family::WindowsPackageFamily;

    fn application(hosts: &[&str]) -> NativeApplication {
        let root = tempfile::tempdir().expect("project");
        let mut application = NativeApplication::discover(root.path()).expect("application");
        application.application_id = String::from("com.example.shop-app");
        application.links = NativeLinks::declared(&["shop"], hosts);
        application
    }

    fn certificate() -> AndroidSigningCertificate {
        AndroidSigningCertificate::parse(Some(&vec!["AB"; 32].join(":"))).expect("certificate")
    }

    #[test]
    fn listed_hosts_get_both_association_files_once_the_signing_values_are_set() {
        let team = AppleTeam::parse(Some("A1B2C3D4E5")).expect("team");
        let application = application(&["example.com"]);
        let (apple, apple_note) = SiteAssociation::apple(&application, team.as_ref());
        let (path, bytes) = apple.expect("apple file");
        assert_eq!(
            path.to_str(),
            Some(".well-known/apple-app-site-association")
        );
        let association: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(
            association["applinks"]["details"][0]["appIDs"][0],
            "A1B2C3D4E5.com.example.shop-app"
        );
        assert_eq!(apple_note, None);
        let (android, android_note) =
            SiteAssociation::android(&application, Some("https://example.com"), &certificate());
        let (path, bytes) = android.expect("android file");
        assert_eq!(path.to_str(), Some(".well-known/assetlinks.json"));
        let statements: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(
            statements[0]["target"]["package_name"],
            "com.example.shop_app"
        );
        assert_eq!(
            statements[0]["target"]["sha256_cert_fingerprints"][0],
            vec!["AB"; 32].join(":")
        );
        assert_eq!(android_note, None);
        let family = WindowsPackageFamily::parse(
            &application.application_id,
            Some("CN=Contoso Ltd, O=Contoso Ltd, C=US"),
        )
        .expect("publisher");
        let (windows, windows_note) = SiteAssociation::windows(&application, family.as_ref());
        let (path, bytes) = windows.expect("windows file");
        assert_eq!(path.to_str(), Some(".well-known/windows-app-web-link"));
        let link: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(
            link[0]["packageFamilyName"],
            "com.example.shop-app_t247dhg0dta00"
        );
        assert_eq!(link[0]["paths"][0], "*");
        assert_eq!(windows_note, None);
    }

    #[test]
    fn listed_hosts_without_the_signing_values_get_a_note_instead() {
        let application = application(&["example.com"]);
        let (apple, apple_note) = SiteAssociation::apple(&application, None);
        assert!(apple.is_none());
        assert!(apple_note.is_some_and(|note| note.contains("TAC_APPLE_TEAM_ID")));
        let (android, android_note) = SiteAssociation::android(&application, None, &certificate());
        assert!(android.is_none());
        assert!(android_note.is_some_and(|note| note.contains("TAC_ANDROID_CERT_SHA256")));
        let (windows, windows_note) = SiteAssociation::windows(&application, None);
        assert!(windows.is_none());
        assert!(
            windows_note.is_some_and(|note| note.contains("TAC_WINDOWS_CERTIFICATE_PUBLISHER"))
        );
    }

    #[test]
    fn an_app_without_hosts_writes_and_notes_nothing() {
        let team = AppleTeam::parse(Some("A1B2C3D4E5")).expect("team");
        let application = application(&[]);
        assert_eq!(
            SiteAssociation::apple(&application, team.as_ref()),
            (None, None)
        );
        assert_eq!(
            SiteAssociation::android(&application, Some("https://example.com"), &certificate()),
            (None, None)
        );
        let family =
            WindowsPackageFamily::parse("com.example.shop", Some("CN=Shop")).expect("publisher");
        assert_eq!(
            SiteAssociation::windows(&application, family.as_ref()),
            (None, None)
        );
    }
}
