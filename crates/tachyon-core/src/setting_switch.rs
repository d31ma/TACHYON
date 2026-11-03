pub(crate) struct SettingSwitch;

impl SettingSwitch {
    const TURNED_ON: [&'static str; 2] = ["1", "true"];

    pub(crate) fn is_set(name: &str) -> bool {
        Self::is_on(std::env::var(name).ok().as_deref())
    }

    pub(crate) fn is_on(configured: Option<&str>) -> bool {
        configured.is_some_and(|value| {
            Self::TURNED_ON
                .iter()
                .any(|on| value.trim().eq_ignore_ascii_case(on))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::SettingSwitch;

    #[test]
    fn a_switch_is_off_unless_turned_on() {
        for (configured, on) in [
            (None, false),
            (Some(""), false),
            (Some("0"), false),
            (Some("yes please"), false),
            (Some("1"), true),
            (Some(" TRUE "), true),
        ] {
            assert_eq!(SettingSwitch::is_on(configured), on, "{configured:?}");
        }
    }
}
