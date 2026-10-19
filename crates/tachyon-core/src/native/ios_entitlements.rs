use super::apple_team::AppleTeam;
use super::config::NativeApplication;
use super::host::{run_tool, write, xml_escape};
use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct IosEntitlements;

impl IosEntitlements {
    const FILE: &'static str = "TachyonHost.entitlements";
    const CODESIGN: &'static str = "/usr/bin/codesign";
    const SIGN_ARGUMENTS: [&'static str; 4] = ["--force", "--sign", "-", "--timestamp=none"];
    const ENTITLEMENTS_ARGUMENT: &'static str = "--entitlements";
    const DOMAINS_KEY: &'static str = "<key>com.apple.developer.associated-domains</key>";

    pub(super) fn write(
        application: &NativeApplication,
        project: &Path,
    ) -> Result<Option<PathBuf>, Failure> {
        let team = AppleTeam::from_environment()?;
        Self::document(application, team.as_ref())
            .map(|document| {
                let path = project.join(Self::FILE);
                write(&path, document.as_bytes())?;
                Ok(path)
            })
            .transpose()
    }

    pub(super) async fn sign(bundle: &Path, entitlements: Option<&Path>) -> Result<(), Failure> {
        let message = "Application path is not valid Unicode.";
        let bundle = super::macos::unicode_path(bundle, message)?;
        let entitlements = entitlements
            .map(|path| super::macos::unicode_path(path, message))
            .transpose()?;
        let mut arguments = Self::SIGN_ARGUMENTS.to_vec();
        arguments.extend(
            entitlements
                .into_iter()
                .flat_map(|path| [Self::ENTITLEMENTS_ARGUMENT, path]),
        );
        arguments.push(bundle);
        run_tool(Self::CODESIGN, &arguments).await.map(|_| ())
    }

    pub(super) fn associated_domains(application: &NativeApplication) -> String {
        Self::domains_entry(&Self::domains(application))
    }

    fn document(application: &NativeApplication, team: Option<&AppleTeam>) -> Option<String> {
        let team = team.filter(|_| !application.links.hosts().is_empty())?;
        Some(format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>application-identifier</key><string>{application_id}</string>
{key}<array>{domains}</array>
<key>com.apple.developer.team-identifier</key><string>{team_id}</string>
</dict></plist>
"#,
            application_id = xml_escape(&team.app_id(&application.application_id)),
            key = Self::DOMAINS_KEY,
            domains = Self::domains(application),
            team_id = team.id(),
        ))
    }

    fn domains(application: &NativeApplication) -> String {
        application
            .links
            .hosts()
            .iter()
            .map(|host| format!("<string>applinks:{}</string>", xml_escape(host)))
            .collect::<Vec<_>>()
            .concat()
    }

    fn domains_entry(domains: &str) -> String {
        if domains.is_empty() {
            return String::new();
        }
        format!("\n{}<array>{domains}</array>", Self::DOMAINS_KEY)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::IosEntitlements;
    use crate::native::apple_team::AppleTeam;
    use crate::native::config::NativeApplication;
    use crate::native::native_links::NativeLinks;

    fn application(hosts: &[&str]) -> NativeApplication {
        let root = tempfile::tempdir().expect("project");
        let mut application = NativeApplication::discover(root.path()).expect("application");
        application.application_id = String::from("com.example.shop");
        application.links = NativeLinks::declared(&["shop"], hosts);
        application
    }

    #[test]
    fn listed_hosts_and_a_team_become_associated_domains() {
        let team = AppleTeam::parse(Some("A1B2C3D4E5")).expect("team");
        let document = IosEntitlements::document(
            &application(&["example.com", "www.example.com"]),
            team.as_ref(),
        )
        .expect("entitlements");
        assert!(document.contains("<string>A1B2C3D4E5.com.example.shop</string>"));
        assert!(document.contains(
            "<array><string>applinks:example.com</string><string>applinks:www.example.com</string></array>"
        ));
        assert!(
            document.contains(
                "<key>com.apple.developer.team-identifier</key><string>A1B2C3D4E5</string>"
            )
        );
    }

    #[test]
    fn a_device_build_adds_the_listed_hosts_to_its_profile_entitlements() {
        assert_eq!(
            IosEntitlements::associated_domains(&application(&["example.com"])),
            "\n<key>com.apple.developer.associated-domains</key><array><string>applinks:example.com</string></array>"
        );
        assert_eq!(IosEntitlements::associated_domains(&application(&[])), "");
    }

    #[test]
    fn without_hosts_or_a_team_there_are_no_entitlements() {
        let team = AppleTeam::parse(Some("A1B2C3D4E5")).expect("team");
        assert_eq!(
            IosEntitlements::document(&application(&[]), team.as_ref()),
            None
        );
        assert_eq!(
            IosEntitlements::document(&application(&["example.com"]), None),
            None
        );
    }
}
