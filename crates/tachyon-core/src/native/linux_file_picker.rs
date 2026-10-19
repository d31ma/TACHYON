pub(super) struct LinuxFilePicker;

impl LinuxFilePicker {
    const SOURCE: &'static str = include_str!("linux_file_picker.c");
    const GRANT: &'static str = "filePicker";

    pub(super) fn source(grants: &[String]) -> String {
        let granted = grants.iter().any(|grant| grant == Self::GRANT);
        let page_grants = serde_json::to_string(grants).unwrap_or_else(|_| String::from("[]"));
        Self::SOURCE
            .replace(
                "__FILE_PICKER_GRANTED__",
                if granted { "TRUE" } else { "FALSE" },
            )
            .replace(
                "__NATIVE_GRANTS__",
                &super::host::c_string_escape(&page_grants),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::LinuxFilePicker;

    #[test]
    fn linux_file_picker_answers_only_when_the_manifest_grants_it() {
        let granted = LinuxFilePicker::source(&[String::from("filePicker")]);
        assert!(granted.contains("static gboolean g_file_picker_granted = TRUE;"));
        assert!(granted.contains(r#"globalThis.__tachyonNativeGrants = [\"filePicker\"];"#));
        let denied = LinuxFilePicker::source(&[]);
        assert!(denied.contains("static gboolean g_file_picker_granted = FALSE;"));
        assert!(denied.contains("globalThis.__tachyonNativeFilePicker = true;"));
        assert!(denied.contains("globalThis.__tachyonNativeGrants = [];"));
        assert!(!denied.contains("__FILE_PICKER_GRANTED__"));
        assert!(!denied.contains("__NATIVE_GRANTS__"));
    }

    #[test]
    fn linux_file_picker_keeps_the_macos_limits() {
        let source = LinuxFilePicker::source(&[]);
        for limit in [
            "#define TACHYON_FILE_PICKER_BYTE_LIMIT 65536",
            "#define TACHYON_FILE_PICKER_HANDLE_LIMIT 128",
            "#define TACHYON_FILE_PICKER_OWNER_LIMIT 8",
            "#define TACHYON_FILE_PICKER_LIFETIME (300 * G_USEC_PER_SEC)",
            "#define TACHYON_FILE_PICKER_DIALOG_SECONDS 30",
        ] {
            assert!(source.contains(limit), "{limit}");
        }
        assert!(source.contains("gtk_file_dialog_open(dialog, parent, cancellable"));
        assert!(source.contains("tachyon_file_picker_allowed_type"));
    }
}
