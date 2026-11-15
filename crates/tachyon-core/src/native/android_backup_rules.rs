use super::host::write;
use crate::Failure;
use std::path::Path;

pub(super) struct AndroidBackupRules;

impl AndroidBackupRules {
    const FILE: &'static str = "app/src/main/res/xml/tachyon_data_extraction_rules.xml";
    pub(super) const MANIFEST_ATTRIBUTE: &'static str =
        "\n      android:dataExtractionRules=\"@xml/tachyon_data_extraction_rules\"";
    const DOMAINS: [&'static str; 9] = [
        "root",
        "file",
        "database",
        "sharedpref",
        "external",
        "device_root",
        "device_file",
        "device_database",
        "device_sharedpref",
    ];
    const SECTIONS: [&'static str; 2] = ["cloud-backup", "device-transfer"];

    pub(super) fn write(project: &Path) -> Result<(), Failure> {
        write(&project.join(Self::FILE), Self::document().as_bytes())
    }

    fn document() -> String {
        let sections: String = Self::SECTIONS
            .iter()
            .map(|section| Self::section(section))
            .collect();
        format!(
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<data-extraction-rules>\n{sections}</data-extraction-rules>\n"
        )
    }

    fn section(name: &str) -> String {
        let excluded: Vec<String> = Self::DOMAINS
            .iter()
            .map(|domain| format!("    <exclude domain=\"{domain}\" path=\".\"/>\n"))
            .collect();
        let excluded = excluded.concat();
        format!("  <{name}>\n{excluded}  </{name}>\n")
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AndroidBackupRules;

    #[test]
    fn no_app_data_leaves_the_device_in_a_backup_or_a_transfer() {
        let document = AndroidBackupRules::document();
        assert!(document.starts_with("<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<data-extraction-rules>\n  <cloud-backup>\n"));
        assert!(document.contains("  </cloud-backup>\n  <device-transfer>\n"));
        assert!(document.ends_with("  </device-transfer>\n</data-extraction-rules>\n"));
        assert_eq!(
            document
                .matches("<exclude domain=\"root\" path=\".\"/>")
                .count(),
            2
        );
        assert_eq!(
            document
                .matches("<exclude domain=\"device_root\" path=\".\"/>")
                .count(),
            2
        );
        assert_eq!(document.matches("<exclude ").count(), 18);
        assert!(!document.contains("<include"));
    }

    #[test]
    fn the_rules_are_written_where_the_manifest_points() {
        let project = tempfile::tempdir().expect("project");
        AndroidBackupRules::write(project.path()).expect("rules");
        let written = std::fs::read_to_string(
            project
                .path()
                .join("app/src/main/res/xml/tachyon_data_extraction_rules.xml"),
        )
        .expect("written rules");
        assert_eq!(written, AndroidBackupRules::document());
        assert!(
            AndroidBackupRules::MANIFEST_ATTRIBUTE.contains("@xml/tachyon_data_extraction_rules")
        );
    }
}
