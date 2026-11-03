pub(super) struct FilePickerSources;

impl FilePickerSources {
    const CSHARP_UNSUPPORTED: &'static str = r#"Call = (_, _) => "{\"status\":\"unsupported\"}";"#;
    const CSHARP_PAGE: &'static str = "Call = PageCapabilityTransport.Send;";
    const KOTLIN_UNSUPPORTED: &'static str = r#"= { _, _ -> "{\"status\":\"unsupported\"}" }"#;
    const KOTLIN_PAGE: &'static str =
        "= { capability, request -> PageCapabilityTransport.send(capability, request) }";

    pub(super) fn csharp() -> &'static str {
        concat!(
            include_str!("capabilities/FilePickerStatus.cs"),
            include_str!("capabilities/FilePickerResult.cs"),
            include_str!("capabilities/FilePickerOptions.cs"),
            include_str!("capabilities/NativeCapabilityOwner.cs"),
            include_str!("capabilities/NativeCapabilities.cs"),
            include_str!("capabilities/FilePicker.cs")
        )
    }

    pub(super) fn csharp_native() -> String {
        format!(
            "{}{}{}",
            Self::csharp(),
            include_str!("capabilities/NativeCapabilityTransport.cs"),
            include_str!("capabilities/NativeCapabilityRegistration.cs")
        )
    }

    pub(super) fn csharp_page() -> String {
        let source = Self::csharp().replace(Self::CSHARP_UNSUPPORTED, Self::CSHARP_PAGE);
        format!(
            "{source}{}",
            include_str!("capabilities/PageCapabilityTransport.cs")
        )
    }

    pub(super) fn kotlin_page() -> String {
        let source = Self::kotlin().replace(Self::KOTLIN_UNSUPPORTED, Self::KOTLIN_PAGE);
        format!(
            "{source}{}",
            include_str!("capabilities/PageCapabilityTransport.kt")
        )
    }

    pub(super) fn kotlin() -> &'static str {
        concat!(
            include_str!("capabilities/FilePickerStatus.kt"),
            include_str!("capabilities/FilePickerResult.kt"),
            include_str!("capabilities/FilePickerOptions.kt"),
            include_str!("capabilities/NativeCapabilityOwner.kt"),
            include_str!("capabilities/NativeCapabilities.kt"),
            include_str!("capabilities/FilePicker.kt"),
            include_str!("web_apis/DocumentTitle.kt"),
            include_str!("web_apis/DocumentTitleBounds.kt"),
            include_str!("web_apis/TacPageScope.kt"),
            include_str!("web_apis/BrowserDocumentTitle.kt")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FilePickerSources;

    #[test]
    fn page_services_send_file_picker_calls_to_the_page_and_linked_services_keep_their_transport() {
        let csharp = FilePickerSources::csharp_page();
        assert!(
            csharp.contains("Func<string, string, string> Call = PageCapabilityTransport.Send;")
        );
        assert!(csharp.contains("internal static class PageCapabilityTransport"));
        assert!(FilePickerSources::csharp().contains(FilePickerSources::CSHARP_UNSUPPORTED));
        assert!(!FilePickerSources::csharp_native().contains("PageCapabilityTransport"));
        let kotlin = FilePickerSources::kotlin_page();
        assert!(kotlin.contains(
            "var call: (String, String) -> String = { capability, request -> PageCapabilityTransport.send(capability, request) }"
        ));
        assert!(kotlin.contains("object PageCapabilityTransport"));
        assert!(FilePickerSources::kotlin().contains(FilePickerSources::KOTLIN_UNSUPPORTED));
    }
}
