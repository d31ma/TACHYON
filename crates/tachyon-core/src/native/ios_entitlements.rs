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
    const DERQ: &'static str = "/usr/bin/derq";
    const DER_EXTENSION: &'static str = "der";
    const XML_SECTION: &'static str = "__entitlements";
    const DER_SECTION: &'static str = "__ents_der";
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

    pub(super) async fn sign(bundle: &Path) -> Result<(), Failure> {
        let bundle = super::macos::unicode_path(bundle, "Application path is not valid Unicode.")?;
        let mut arguments = Self::SIGN_ARGUMENTS.to_vec();
        arguments.push(bundle);
        run_tool(Self::CODESIGN, &arguments).await.map(|_| ())
    }

    /// Returns the linker arguments that put a simulator app's entitlements
    /// in its executable, as Xcode does: the simulator refuses an ad hoc
    /// signature that carries restricted entitlements.
    ///
    /// # Errors
    ///
    /// Returns a failure when `derq` cannot encode the entitlements.
    pub(super) async fn simulator_sections(
        entitlements: Option<&Path>,
    ) -> Result<Vec<String>, Failure> {
        let Some(xml) = entitlements else {
            return Ok(Vec::new());
        };
        let message = "Entitlements path is not valid Unicode.";
        let der = xml.with_extension(Self::DER_EXTENSION);
        let xml_text = super::macos::unicode_path(xml, message)?;
        let der_text = super::macos::unicode_path(&der, message)?;
        run_tool(
            Self::DERQ,
            &[
                "query", "-f", "xml", "-i", xml_text, "-o", der_text, "--raw",
            ],
        )
        .await?;
        Ok(Self::section_arguments(xml_text, der_text))
    }

    fn section_arguments(xml: &str, der: &str) -> Vec<String> {
        [(Self::XML_SECTION, xml), (Self::DER_SECTION, der)]
            .into_iter()
            .flat_map(|(section, file)| {
                ["-sectcreate", "__TEXT", section, file]
                    .into_iter()
                    .flat_map(|argument| ["-Xlinker", argument])
            })
            .map(String::from)
            .collect()
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
    fn a_simulator_app_links_its_entitlements_into_text_sections() {
        assert_eq!(
            IosEntitlements::section_arguments("app.entitlements", "app.der"),
            [
                "-Xlinker",
                "-sectcreate",
                "-Xlinker",
                "__TEXT",
                "-Xlinker",
                "__entitlements",
                "-Xlinker",
                "app.entitlements",
                "-Xlinker",
                "-sectcreate",
                "-Xlinker",
                "__TEXT",
                "-Xlinker",
                "__ents_der",
                "-Xlinker",
                "app.der"
            ]
        );
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
