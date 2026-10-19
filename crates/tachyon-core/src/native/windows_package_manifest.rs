use super::config::NativeApplication;
use super::host::xml_escape;
use super::native_release_version::NativeReleaseVersion;
use super::windows_architecture::WindowsArchitecture;
use super::windows_package_tiles::WindowsPackageTiles;
use crate::Failure;
use crate::failure::diagnostic;

pub(super) struct WindowsPackageManifest<'application> {
    pub(super) application: &'application NativeApplication,
    pub(super) publisher: &'application str,
    pub(super) architecture: WindowsArchitecture,
}

impl WindowsPackageManifest<'_> {
    pub(super) const FILE: &'static str = "AppxManifest.xml";
    pub(super) const APPLICATION_ID: &'static str = "App";
    const NAME_PUNCTUATION: [char; 2] = ['.', '-'];
    const SHORTEST_NAME: usize = 3;
    const LONGEST_NAME: usize = 50;
    const MINIMUM_WINDOWS: &'static str = "10.0.17763.0";
    const TESTED_WINDOWS: &'static str = "10.0.26100.0";

    pub(super) fn identity_name(application_id: &str) -> Result<String, Failure> {
        let name: String = application_id
            .chars()
            .map(|character| {
                let kept = [
                    character.is_ascii_alphanumeric(),
                    Self::NAME_PUNCTUATION.contains(&character),
                ]
                .contains(&true);
                ['-', character][usize::from(kept)]
            })
            .collect();
        (Self::SHORTEST_NAME..=Self::LONGEST_NAME)
            .contains(&name.len())
            .then_some(name)
            .ok_or_else(|| {
                Failure::one(diagnostic(
                    1601,
                    format!(
                        "Manifest tac.application.id '{application_id}' cannot name the Windows package: \
                         an MSIX package name has 3 to 50 letters, digits, dots and hyphens."
                    ),
                    Some(String::from(
                        "Shorten tac.application.id, or build Windows without --release.",
                    )),
                    None,
                ))
            })
    }

    pub(super) fn publisher_display_name(publisher: &str) -> &str {
        publisher
            .split(',')
            .map(str::trim)
            .find_map(|part| part.strip_prefix("CN="))
            .map_or(publisher, |name| name.trim_matches('"'))
    }

    pub(super) fn xml(&self) -> Result<String, Failure> {
        let application = self.application;
        let name = xml_escape(&application.name);
        Ok(format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
  xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
  xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
  IgnorableNamespaces="uap rescap">
  <Identity Name="{identity}" Publisher="{publisher}" Version="{version}" ProcessorArchitecture="{architecture}"/>
  <Properties>
    <DisplayName>{name}</DisplayName>
    <PublisherDisplayName>{publisher_name}</PublisherDisplayName>
    <Logo>{store_logo}</Logo>
  </Properties>
  <Dependencies>
    <TargetDeviceFamily Name="Windows.Desktop" MinVersion="{minimum}" MaxVersionTested="{tested}"/>
  </Dependencies>
  <Resources>
    <Resource Language="en-us"/>
  </Resources>
  <Applications>
    <Application Id="{application_id}" Executable="bin\{executable}.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="{name}" Description="{name}" BackgroundColor="{background}"
        Square150x150Logo="{square_150}" Square44x44Logo="{square_44}"/>
    </Application>
  </Applications>
  <Capabilities>
    <rescap:Capability Name="runFullTrust"/>
  </Capabilities>
</Package>
"#,
            identity = Self::identity_name(&application.application_id)?,
            publisher = xml_escape(self.publisher),
            version = NativeReleaseVersion::of(application).windows_dotted(),
            architecture = self.architecture.name(),
            publisher_name = xml_escape(Self::publisher_display_name(self.publisher)),
            store_logo = WindowsPackageTiles::STORE_LOGO.manifest_path(),
            minimum = Self::MINIMUM_WINDOWS,
            tested = Self::TESTED_WINDOWS,
            application_id = Self::APPLICATION_ID,
            executable = xml_escape(&application.executable_name),
            background = Self::background(application),
            square_150 = WindowsPackageTiles::SQUARE_150.manifest_path(),
            square_44 = WindowsPackageTiles::SQUARE_44.manifest_path(),
        ))
    }

    fn background(application: &NativeApplication) -> String {
        application.window.theme_color.map_or_else(
            || String::from("transparent"),
            |color| format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue),
        )
    }
}
