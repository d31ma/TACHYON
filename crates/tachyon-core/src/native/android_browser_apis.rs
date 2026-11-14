use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct AndroidBrowserApis;

impl AndroidBrowserApis {
    const JAVA: &'static str = include_str!("native_browser_apis_android.java");
    const ANSWERED: [&'static str; 7] = [
        "camera",
        "microphone",
        "clipboard",
        "share",
        "notifications",
        "geolocation",
        "persistentStorage",
    ];
    const MANIFEST_PERMISSIONS: [(&'static str, &'static str); 6] = [
        ("camera", "android.permission.CAMERA"),
        ("geolocation", "android.permission.ACCESS_COARSE_LOCATION"),
        ("geolocation", "android.permission.ACCESS_FINE_LOCATION"),
        ("microphone", "android.permission.RECORD_AUDIO"),
        ("microphone", "android.permission.MODIFY_AUDIO_SETTINGS"),
        ("notifications", "android.permission.POST_NOTIFICATIONS"),
    ];
    const MANIFEST_FEATURES: [(&'static str, &'static str); 1] =
        [("camera", "android.hardware.camera.any")];
    const PERMISSION_LINE: [&'static str; 2] = ["\n  <uses-permission android:name=\"", "\"/>"];
    const FEATURE_LINE: [&'static str; 2] = [
        "\n  <uses-feature android:name=\"",
        "\" android:required=\"false\"/>",
    ];

    pub(super) fn java(permissions: &BrowserApiPermissions) -> String {
        Self::JAVA.replace("__BROWSER_API_GRANTS__", &Self::java_grants(permissions))
    }

    #[cfg(test)]
    pub(super) const fn template() -> &'static str {
        Self::JAVA
    }

    pub(super) fn manifest_permissions(permissions: &BrowserApiPermissions) -> String {
        [
            Self::manifest_lines(
                permissions,
                &Self::MANIFEST_PERMISSIONS,
                Self::PERMISSION_LINE,
            ),
            Self::manifest_lines(permissions, &Self::MANIFEST_FEATURES, Self::FEATURE_LINE),
        ]
        .concat()
    }

    fn manifest_lines(
        permissions: &BrowserApiPermissions,
        entries: &[(&str, &str)],
        line: [&str; 2],
    ) -> String {
        entries
            .iter()
            .filter(|(api, _)| permissions.is_granted(api))
            .map(|(_, name)| [line[0], name, line[1]].concat())
            .collect()
    }

    fn java_grants(permissions: &BrowserApiPermissions) -> String {
        let quoted: Vec<String> = Self::ANSWERED
            .iter()
            .filter(|api| permissions.is_granted(api))
            .map(|api| format!("\"{api}\""))
            .collect();
        format!("{{{}}}", quoted.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::AndroidBrowserApis;
    use crate::native::browser_api_permissions::BrowserApiPermissions;
    use std::collections::BTreeMap;

    fn granting(apis: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = apis.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &BTreeMap::new())
    }

    #[test]
    fn the_host_answers_only_the_granted_browser_apis() {
        let java =
            AndroidBrowserApis::java(&granting(&["share", "preferenceStrings", "clipboard"]));
        assert!(java.contains(
            "private static final String[] BROWSER_API_GRANTS = {\"clipboard\", \"share\"};"
        ));
        assert!(!java.contains("__BROWSER_API_GRANTS__"));
        assert!(
            AndroidBrowserApis::java(&granting(&[]))
                .contains("private static final String[] BROWSER_API_GRANTS = {};")
        );
    }

    #[test]
    fn a_page_file_input_opens_the_system_chooser_and_hears_its_answer() {
        let java = AndroidBrowserApis::java(&granting(&[]));
        assert!(java.contains("public boolean onShowFileChooser("));
        assert!(
            java.contains("startActivityForResult(parameters.createIntent(), PAGE_FILE_REQUEST);")
        );
        assert!(java.contains("FileChooserParams.parseResult(resultCode, data)"));
        let picker = crate::native::android_file_picker::AndroidFilePicker::java(&[]);
        assert!(
            picker.contains("if (answerPageFileChooser(requestCode, resultCode, data)) return;")
        );
    }

    #[test]
    fn the_manifest_asks_android_only_for_the_permissions_of_granted_apis() {
        assert_eq!(
            AndroidBrowserApis::manifest_permissions(&granting(&["clipboard", "share"])),
            ""
        );
        assert_eq!(
            AndroidBrowserApis::manifest_permissions(&granting(&["geolocation"])),
            "\n  <uses-permission android:name=\"android.permission.ACCESS_COARSE_LOCATION\"/>\n  <uses-permission android:name=\"android.permission.ACCESS_FINE_LOCATION\"/>"
        );
        assert_eq!(
            AndroidBrowserApis::manifest_permissions(&granting(&["notifications"])),
            "\n  <uses-permission android:name=\"android.permission.POST_NOTIFICATIONS\"/>"
        );
        assert_eq!(
            AndroidBrowserApis::manifest_permissions(&granting(&["camera"])),
            "\n  <uses-permission android:name=\"android.permission.CAMERA\"/>\n  <uses-feature android:name=\"android.hardware.camera.any\" android:required=\"false\"/>"
        );
    }

    #[test]
    fn location_goes_through_the_web_view_prompt_and_the_runtime_request() {
        let java = AndroidBrowserApis::template();
        assert!(java.contains("public void onGeolocationPermissionsShowPrompt("));
        assert!(java.contains(
            "view.getSettings().setGeolocationEnabled(browserApiGranted(\"geolocation\"));"
        ));
        assert!(java.contains("\"android.permission.ACCESS_FINE_LOCATION\""));
        assert!(java.contains("requestPermissions(permissions, requestCode);"));
        assert!(java.contains("shouldShowRequestPermissionRationale(permissions[0])"));
        assert!(java.contains("\"tachyon_permission_\" + api"));
    }

    #[test]
    fn the_camera_goes_only_to_the_app_pages_after_the_runtime_request() {
        let java = AndroidBrowserApis::template();
        assert!(
            java.contains(
                "public void onPermissionRequest(android.webkit.PermissionRequest request)"
            )
        );
        assert!(java.contains("{android.webkit.PermissionRequest.RESOURCE_VIDEO_CAPTURE, \"camera\", \"android.permission.CAMERA\"},"));
        assert!(java.contains(
            "if (origin == null || !isAssetUrl(origin) || resources.length == 0) return null;"
        ));
        assert!(
            java.contains("if (row[0].equals(resource) && browserApiGranted(row[1])) return row;")
        );
        assert!(
            java.contains("askAndroidPermission(CAPTURE_PERMISSION_REQUEST, missing, rows[0][1],")
        );
        assert!(java.contains("request.deny();"));
    }

    #[test]
    fn notifications_ask_on_android_13_and_post_on_a_channel_named_from_the_manifest() {
        let java = AndroidBrowserApis::template();
        assert!(java.contains("android.os.Build.VERSION.SDK_INT >= 33"));
        assert!(java.contains("\"android.permission.POST_NOTIFICATIONS\""));
        assert!(java.contains("NotificationManagerCompat.from(this).areNotificationsEnabled()"));
        assert!(java.contains("reason == null ? getString(R.string.app_name) : reason"));
        assert!(java.contains("globalThis.__tachyonNotificationPermission = "));
        assert!(java.contains("WebViewFeature.DOCUMENT_START_SCRIPT"));
    }

    #[test]
    fn the_microphone_goes_only_to_the_app_pages_with_its_two_permissions() {
        let java = AndroidBrowserApis::template();
        assert!(java.contains("{android.webkit.PermissionRequest.RESOURCE_AUDIO_CAPTURE, \"microphone\", \"android.permission.RECORD_AUDIO\"},"));
        let microphone = granting(&["microphone"]);
        assert_eq!(
            AndroidBrowserApis::manifest_permissions(&microphone),
            "\n  <uses-permission android:name=\"android.permission.RECORD_AUDIO\"/>\n  <uses-permission android:name=\"android.permission.MODIFY_AUDIO_SETTINGS\"/>"
        );
        assert!(AndroidBrowserApis::java(&microphone).contains("{\"microphone\"}"));
    }
}
