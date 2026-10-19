use super::browser_api_permissions::BrowserApiPermissions;

pub(super) struct AndroidBrowserApis;

impl AndroidBrowserApis {
    const JAVA: &'static str = include_str!("native_browser_apis_android.java");
    const ANSWERED: [&'static str; 4] = ["clipboard", "share", "notifications", "geolocation"];
    const MANIFEST_PERMISSIONS: [(&'static str, &'static str); 3] = [
        ("geolocation", "android.permission.ACCESS_COARSE_LOCATION"),
        ("geolocation", "android.permission.ACCESS_FINE_LOCATION"),
        ("notifications", "android.permission.POST_NOTIFICATIONS"),
    ];

    pub(super) fn java(permissions: &BrowserApiPermissions) -> String {
        Self::JAVA.replace("__BROWSER_API_GRANTS__", &Self::java_grants(permissions))
    }

    #[cfg(test)]
    pub(super) const fn template() -> &'static str {
        Self::JAVA
    }

    pub(super) fn manifest_permissions(permissions: &BrowserApiPermissions) -> String {
        Self::MANIFEST_PERMISSIONS
            .iter()
            .filter(|(api, _)| permissions.is_granted(api))
            .map(|(_, permission)| {
                ["\n  <uses-permission android:name=\"", permission, "\"/>"].concat()
            })
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
    fn notifications_ask_on_android_13_and_post_on_a_channel_named_from_the_manifest() {
        let java = AndroidBrowserApis::template();
        assert!(java.contains("android.os.Build.VERSION.SDK_INT >= 33"));
        assert!(java.contains("\"android.permission.POST_NOTIFICATIONS\""));
        assert!(java.contains("NotificationManagerCompat.from(this).areNotificationsEnabled()"));
        assert!(java.contains("reason == null ? getString(R.string.app_name) : reason"));
        assert!(java.contains("globalThis.__tachyonNotificationPermission = "));
        assert!(java.contains("WebViewFeature.DOCUMENT_START_SCRIPT"));
    }
}
