use super::browser_api_permissions::BrowserApiPermissions;
use super::host::quoted_string_escape;
use super::media_member::MediaMember;

pub(super) struct AndroidCallAudio;

impl AndroidCallAudio {
    const JAVA: &'static str = include_str!("android_call_audio.java");
    const PERMISSION_LINE: [&'static str; 2] = ["\n  <uses-permission android:name=\"", "\"/>"];
    const SERVICE_PERMISSION: &'static str = "android.permission.FOREGROUND_SERVICE";
    const KIND_PERMISSIONS: [(&'static str, &'static str); 2] = [
        (
            "microphone",
            "android.permission.FOREGROUND_SERVICE_MICROPHONE",
        ),
        ("camera", "android.permission.FOREGROUND_SERVICE_CAMERA"),
    ];
    const NOTIFICATION_PERMISSION: &'static str = "android.permission.POST_NOTIFICATIONS";
    const NOTIFICATIONS: &'static str = "notifications";
    const SERVICE_START: &'static str = "\n    <service android:name=\".MainActivity$CallService\" android:exported=\"false\" android:foregroundServiceType=\"";
    const SERVICE_END: &'static str = "\"/>";
    const MEDIA_GRANTS: [&'static str; 2] = ["microphone", "camera"];

    pub(super) fn java(media: &MediaMember) -> String {
        Self::JAVA
            .replace(
                "__CALL_NOTICE__",
                &quoted_string_escape(&media.call_notice()),
            )
            .replace("__CALL_BACKGROUND__", Self::java_flag(media.background()))
    }

    pub(super) fn manifest_permissions(
        media: &MediaMember,
        permissions: &BrowserApiPermissions,
    ) -> String {
        let lines = Self::background_permissions(permissions);
        [String::new(), lines][usize::from(media.background())].clone()
    }

    pub(super) fn service_entry(
        media: &MediaMember,
        permissions: &BrowserApiPermissions,
    ) -> String {
        let types = Self::service_types(permissions);
        let entry = [Self::SERVICE_START, &types, Self::SERVICE_END].concat();
        [String::new(), entry][usize::from(media.background())].clone()
    }

    fn background_permissions(permissions: &BrowserApiPermissions) -> String {
        let kinds = Self::KIND_PERMISSIONS
            .iter()
            .filter(|(api, _)| permissions.is_granted(api))
            .map(|(_, name)| *name);
        let notification = Self::notification_permission(permissions);
        std::iter::once(Self::SERVICE_PERMISSION)
            .chain(kinds)
            .chain(notification)
            .map(|name| [Self::PERMISSION_LINE[0], name, Self::PERMISSION_LINE[1]].concat())
            .collect()
    }

    fn notification_permission(permissions: &BrowserApiPermissions) -> Option<&'static str> {
        (!permissions.is_granted(Self::NOTIFICATIONS)).then_some(Self::NOTIFICATION_PERMISSION)
    }

    fn service_types(permissions: &BrowserApiPermissions) -> String {
        let granted: Vec<&str> = Self::MEDIA_GRANTS
            .into_iter()
            .filter(|api| permissions.is_granted(api))
            .collect();
        granted.join("|")
    }

    fn java_flag(on: bool) -> &'static str {
        ["false", "true"][usize::from(on)]
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::AndroidCallAudio;
    use crate::native::browser_api_permissions::BrowserApiPermissions;
    use crate::native::media_member::MediaMember;

    fn media(json: &str) -> MediaMember {
        serde_json::from_str(json).expect("media")
    }

    fn granting(apis: &[&str]) -> BrowserApiPermissions {
        let granted: Vec<String> = apis.iter().map(|api| String::from(*api)).collect();
        BrowserApiPermissions::declared(&granted, &std::collections::BTreeMap::new())
    }

    #[test]
    fn a_background_call_declares_its_service_and_permissions() {
        let calling = media(r#"{"background": true, "callNotice": "Call \"live\""}"#);
        let both = granting(&["microphone", "camera"]);
        assert_eq!(
            AndroidCallAudio::manifest_permissions(&calling, &both),
            "\n  <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE\"/>\n  <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE_MICROPHONE\"/>\n  <uses-permission android:name=\"android.permission.FOREGROUND_SERVICE_CAMERA\"/>\n  <uses-permission android:name=\"android.permission.POST_NOTIFICATIONS\"/>"
        );
        assert_eq!(
            AndroidCallAudio::service_entry(&calling, &both),
            "\n    <service android:name=\".MainActivity$CallService\" android:exported=\"false\" android:foregroundServiceType=\"microphone|camera\"/>"
        );
        let notifying = granting(&["microphone", "notifications"]);
        assert!(
            !AndroidCallAudio::manifest_permissions(&calling, &notifying)
                .contains("POST_NOTIFICATIONS"),
            "the notifications grant already declares it"
        );
        let java = AndroidCallAudio::java(&calling);
        assert!(java.contains(r#"CALL_NOTICE = "Call \"live\"";"#), "{java}");
        assert!(java.contains("CALL_BACKGROUND = true;"));
        assert!(java.contains("case \"browserApi.media.callActive\""));
        assert!(java.contains("startForeground(CALL_NOTIFICATION, shown, callTypes(camera))"));
    }

    #[test]
    fn without_background_audio_no_service_and_no_permission_is_declared() {
        let quiet = MediaMember::default();
        let both = granting(&["microphone", "camera"]);
        assert_eq!(AndroidCallAudio::manifest_permissions(&quiet, &both), "");
        assert_eq!(AndroidCallAudio::service_entry(&quiet, &both), "");
        let java = AndroidCallAudio::java(&quiet);
        assert!(java.contains("CALL_BACKGROUND = false;"));
        assert!(java.contains(r#"CALL_NOTICE = "";"#));
    }
}
