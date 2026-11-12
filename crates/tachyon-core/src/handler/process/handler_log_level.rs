#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub(in crate::handler) enum HandlerLogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Off,
}

impl HandlerLogLevel {
    const SETTING: &'static str = "TAC_HANDLER_LOG_LEVEL";
    const LINE_LEVELS: [(&'static str, Self); 4] = [
        ("debug", Self::Debug),
        ("info", Self::Info),
        ("warn", Self::Warn),
        ("error", Self::Error),
    ];
    const SETTING_LEVELS: [(&'static str, Self); 5] = [
        ("debug", Self::Debug),
        ("info", Self::Info),
        ("warn", Self::Warn),
        ("error", Self::Error),
        ("off", Self::Off),
    ];

    pub(in crate::handler) fn configured() -> Self {
        Self::from_setting(std::env::var(Self::SETTING).ok().as_deref())
    }

    pub(in crate::handler) fn from_setting(value: Option<&str>) -> Self {
        let name = value.unwrap_or_default().trim().to_ascii_lowercase();
        Self::find(&Self::SETTING_LEVELS, &name).unwrap_or(Self::Info)
    }

    pub(in crate::handler) fn of_line(name: &str) -> Option<Self> {
        Self::find(&Self::LINE_LEVELS, name)
    }

    pub(in crate::handler) fn reaches(self, threshold: Self) -> bool {
        self >= threshold
    }

    fn find(levels: &[(&'static str, Self)], name: &str) -> Option<Self> {
        levels
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, level)| *level)
    }
}

#[cfg(test)]
mod tests {
    use super::HandlerLogLevel;

    #[test]
    fn the_setting_picks_the_lowest_level_and_keeps_info_otherwise() {
        assert_eq!(HandlerLogLevel::from_setting(None), HandlerLogLevel::Info);
        assert_eq!(
            HandlerLogLevel::from_setting(Some(" DEBUG ")),
            HandlerLogLevel::Debug
        );
        assert_eq!(
            HandlerLogLevel::from_setting(Some("warn")),
            HandlerLogLevel::Warn
        );
        assert_eq!(
            HandlerLogLevel::from_setting(Some("off")),
            HandlerLogLevel::Off
        );
        assert_eq!(
            HandlerLogLevel::from_setting(Some("loud")),
            HandlerLogLevel::Info
        );
    }

    #[test]
    fn a_line_level_is_one_of_four_lower_case_names() {
        assert_eq!(
            HandlerLogLevel::of_line("error"),
            Some(HandlerLogLevel::Error)
        );
        assert_eq!(HandlerLogLevel::of_line("off"), None);
        assert_eq!(HandlerLogLevel::of_line("INFO"), None);
    }

    #[test]
    fn off_hides_every_level_and_debug_shows_every_level() {
        assert!(!HandlerLogLevel::Error.reaches(HandlerLogLevel::Off));
        assert!(HandlerLogLevel::Debug.reaches(HandlerLogLevel::Debug));
        assert!(!HandlerLogLevel::Debug.reaches(HandlerLogLevel::Info));
        assert!(HandlerLogLevel::Warn.reaches(HandlerLogLevel::Info));
    }
}
