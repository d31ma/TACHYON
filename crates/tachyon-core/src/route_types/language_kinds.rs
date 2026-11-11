pub(super) struct LanguageKinds {
    pub(super) text: &'static str,
    pub(super) number: &'static str,
    pub(super) flag: &'static str,
}

impl LanguageKinds {
    pub(super) const fn new(text: &'static str, number: &'static str, flag: &'static str) -> Self {
        Self { text, number, flag }
    }
}
