pub(super) struct FilePickerSources;

impl FilePickerSources {
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
