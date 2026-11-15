use super::config::config_failure;
use crate::Failure;
use serde::Deserialize;
use tachyon_contracts::NativeTarget;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(crate) struct MediaMember {
    #[serde(default)]
    background: bool,
    #[serde(default)]
    call_notice: Option<String>,
}

impl MediaMember {
    const MOST_CHARACTERS: usize = 100;
    const NO_ENTRY: &'static str = "";
    const BACKGROUND_SCRIPT: &'static str = "globalThis.__tachyonMediaBackground = true;";
    const AUDIO_MODE: &'static str =
        "\n<key>UIBackgroundModes</key><array><string>audio</string></array>";
    const NOTICE_MESSAGE: &'static str = "Manifest tac.media.callNotice is 1 to 100 characters of plain text, such as \"Call in progress\".";
    const ANDROID_MESSAGE: &'static str = "An Android build with tac.media.background true needs tac.media.callNotice, the text of the notification Android shows while a call runs in the background.";

    pub(crate) fn validate(&self) -> Result<(), Failure> {
        self.call_notice
            .iter()
            .try_for_each(|notice| Self::check_notice(notice))
    }

    pub(crate) fn validate_for(&self, target: NativeTarget) -> Result<(), Failure> {
        let missing = [
            self.background,
            target == NativeTarget::Android,
            self.call_notice.is_none(),
        ] == [true, true, true];
        (!missing)
            .then_some(())
            .ok_or_else(|| config_failure(Self::ANDROID_MESSAGE))
    }

    pub(crate) fn background(&self) -> bool {
        self.background
    }

    pub(crate) fn call_notice(&self) -> String {
        self.call_notice.clone().unwrap_or_default()
    }

    pub(crate) fn page_script(&self) -> &'static str {
        [Self::NO_ENTRY, Self::BACKGROUND_SCRIPT][usize::from(self.background)]
    }

    pub(crate) fn plist_entries(&self) -> &'static str {
        [Self::NO_ENTRY, Self::AUDIO_MODE][usize::from(self.background)]
    }

    fn check_notice(notice: &str) -> Result<(), Failure> {
        let plain = [
            notice.trim().is_empty(),
            notice.chars().count() > Self::MOST_CHARACTERS,
            notice.chars().any(char::is_control),
        ] == [false, false, false];
        plain
            .then_some(())
            .ok_or_else(|| config_failure(Self::NOTICE_MESSAGE))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::MediaMember;
    use tachyon_contracts::NativeTarget;

    fn member(json: &str) -> Result<MediaMember, serde_json::Error> {
        serde_json::from_str(json)
    }

    #[test]
    fn a_background_call_needs_a_notice_only_on_android() {
        let silent = member(r#"{"background": true}"#).expect("member");
        silent.validate().expect("no notice is valid");
        let error = format!(
            "{:?}",
            silent
                .validate_for(NativeTarget::Android)
                .expect_err("refused")
        );
        assert!(error.contains("tac.media.callNotice"), "{error}");
        for target in [
            NativeTarget::Ios,
            NativeTarget::Macos,
            NativeTarget::Windows,
            NativeTarget::Linux,
        ] {
            silent.validate_for(target).expect("no notice needed");
        }
        let noticed =
            member(r#"{"background": true, "callNotice": "Call in progress"}"#).expect("member");
        noticed
            .validate_for(NativeTarget::Android)
            .expect("explained");
        assert_eq!(noticed.call_notice(), "Call in progress");
        assert!(noticed.background());
    }

    #[test]
    fn the_notice_is_short_plain_text_and_unknown_fields_are_refused() {
        for notice in ["\"\"", "\"  \"", "\"line\\nbreak\""] {
            let given = member(&format!(r#"{{"callNotice": {notice}}}"#)).expect("member");
            given.validate().expect_err("refused");
        }
        let long = member(&format!(r#"{{"callNotice": "{}"}}"#, "x".repeat(101))).expect("member");
        long.validate().expect_err("too long");
        assert!(member(r#"{"backgroundAudio": true}"#).is_err());
    }

    #[test]
    fn only_background_audio_adds_the_ios_background_mode() {
        let quiet = MediaMember::default();
        assert_eq!(quiet.plist_entries(), "");
        assert!(!quiet.background());
        let calling = member(r#"{"background": true}"#).expect("member");
        assert_eq!(
            calling.plist_entries(),
            "\n<key>UIBackgroundModes</key><array><string>audio</string></array>"
        );
    }
}
