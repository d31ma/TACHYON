use super::config::NativeApplication;
use super::native_release_version::NativeReleaseVersion;

pub(super) struct WindowsVersionResource<'application> {
    pub(super) application: &'application NativeApplication,
    pub(super) has_icon: bool,
}

impl WindowsVersionResource<'_> {
    const ICON: &'static str = "1 ICON \"icon.ico\"\n";

    pub(super) fn script(&self) -> String {
        let release = NativeReleaseVersion::of(self.application);
        let numbers = release
            .windows_numbers()
            .map(|number| number.to_string())
            .join(",");
        let icon = ["", Self::ICON][usize::from(self.has_icon)];
        format!(
            r#"#pragma code_page(65001)
1 24 "application.manifest"
{icon}1 VERSIONINFO
FILEVERSION {numbers}
PRODUCTVERSION {numbers}
FILEFLAGSMASK 0x3F
FILEFLAGS 0x0
FILEOS 0x40004
FILETYPE 0x1
FILESUBTYPE 0x0
BEGIN
  BLOCK "StringFileInfo"
  BEGIN
    BLOCK "040904B0"
    BEGIN
      VALUE "FileDescription", "{name}"
      VALUE "FileVersion", "{file_version}"
      VALUE "InternalName", "{executable}"
      VALUE "OriginalFilename", "{executable}.exe"
      VALUE "ProductName", "{name}"
      VALUE "ProductVersion", "{product_version}"
    END
  END
  BLOCK "VarFileInfo"
  BEGIN
    VALUE "Translation", 0x409, 1200
  END
END
"#,
            name = Self::quoted(&self.application.name),
            file_version = release.windows_dotted(),
            executable = self.application.executable_name,
            product_version = Self::quoted(release.version()),
        )
    }

    fn quoted(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\"\"")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::super::config::NativeApplication;
    use super::WindowsVersionResource;

    fn application(name: &str, version: &str, build: u32) -> NativeApplication {
        let root = tempfile::tempdir().expect("project");
        std::fs::create_dir_all(root.path().join("client")).expect("client");
        let manifest = serde_json::json!({
            "name": name,
            "tac": { "application": { "version": version, "build": build } },
        });
        std::fs::write(
            root.path().join("client/manifest.json"),
            manifest.to_string(),
        )
        .expect("manifest");
        NativeApplication::discover(root.path()).expect("application")
    }

    #[test]
    fn the_version_resource_carries_the_version_and_the_build() {
        let application = application("Catalog", "2.5", 31);
        let script = WindowsVersionResource {
            application: &application,
            has_icon: false,
        }
        .script();
        assert!(script.starts_with(
            "#pragma code_page(65001)\n1 24 \"application.manifest\"\n1 VERSIONINFO\n"
        ));
        assert!(script.contains("FILEVERSION 2,5,0,31\n"), "{script}");
        assert!(script.contains("PRODUCTVERSION 2,5,0,31\n"), "{script}");
        assert!(
            script.contains(r#"VALUE "FileVersion", "2.5.0.31""#),
            "{script}"
        );
        assert!(
            script.contains(r#"VALUE "ProductVersion", "2.5""#),
            "{script}"
        );
        assert!(script.contains(r#"VALUE "OriginalFilename", "Catalog.exe""#));
        assert!(!script.contains("ICON"), "{script}");
    }

    #[test]
    fn the_icon_and_a_quoted_name_stay_inside_the_script() {
        let application = application(r#"Say "Hi" \ Café"#, "1.0.0", 1);
        let script = WindowsVersionResource {
            application: &application,
            has_icon: true,
        }
        .script();
        assert!(
            script
                .contains("\n1 24 \"application.manifest\"\n1 ICON \"icon.ico\"\n1 VERSIONINFO\n"),
            "{script}"
        );
        assert!(
            script.contains(r#"VALUE "ProductName", "Say ""Hi"" \\ Café""#),
            "{script}"
        );
    }
}
