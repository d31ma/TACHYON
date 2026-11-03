pub(super) struct AndroidFilePicker;

impl AndroidFilePicker {
    const SOURCE: &'static str = include_str!("native_file_picker_android.java");
    const GRANT: &'static str = "filePicker";
    const GRANTED: [&'static str; 2] = ["false", "true"];

    pub(super) fn java(grants: &[String]) -> String {
        let granted = grants.iter().any(|grant| grant == Self::GRANT);
        Self::SOURCE.replace(
            "__FILE_PICKER_GRANTED__",
            Self::GRANTED[usize::from(granted)],
        )
    }

    pub(super) fn page_script(grants: &[String]) -> String {
        let page_grants = serde_json::to_string(grants).unwrap_or_else(|_| String::from("[]"));
        format!(
            "\nglobalThis.__tachyonNativeFilePicker = true; globalThis.__tachyonNativeFilePickerRoute = true; globalThis.__tachyonNativeGrants = {page_grants};"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AndroidFilePicker;

    #[test]
    fn the_android_file_picker_answers_only_when_the_manifest_grants_it() {
        let granted = AndroidFilePicker::java(&[String::from("filePicker")]);
        assert!(granted.contains("private static final boolean FILE_PICKER_GRANTED = true;"));
        let denied = AndroidFilePicker::java(&[]);
        assert!(denied.contains("private static final boolean FILE_PICKER_GRANTED = false;"));
        assert!(!denied.contains("__FILE_PICKER_GRANTED__"));
        assert!(
            AndroidFilePicker::page_script(&[String::from("filePicker")])
                .ends_with(r#"globalThis.__tachyonNativeGrants = ["filePicker"];"#)
        );
        assert!(
            AndroidFilePicker::page_script(&[]).ends_with("globalThis.__tachyonNativeGrants = [];")
        );
    }

    #[test]
    fn the_android_file_picker_keeps_the_desktop_limits_and_opens_the_storage_access_framework() {
        let source = AndroidFilePicker::java(&[]);
        for limit in [
            "FILE_PICKER_BYTE_LIMIT = 65536;",
            "FILE_PICKER_HANDLE_LIMIT = 128;",
            "FILE_PICKER_OWNER_LIMIT = 8;",
            "FILE_PICKER_LIFETIME = 300000L;",
            "FILE_PICKER_DIALOG_MILLISECONDS = 30000L;",
            "new Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE)",
            "startActivityForResult(intent, FILE_PICKER_REQUEST)",
            "finishActivity(FILE_PICKER_REQUEST)",
            "if (android.os.Looper.myLooper() == android.os.Looper.getMainLooper()) return filePickerStatus(\"unavailable\");",
            ".onMalformedInput(java.nio.charset.CodingErrorAction.REPORT)",
        ] {
            assert!(source.contains(limit), "{limit}");
        }
    }
}
