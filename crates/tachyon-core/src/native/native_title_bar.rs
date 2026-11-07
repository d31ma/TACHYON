use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(super) enum NativeTitleBar {
    #[default]
    Standard,
    Integrated,
}

impl NativeTitleBar {
    const MOBILE_PAGE_SCRIPT: &'static str = include_str!("title_bar/tac-mobile-title-bar.js");

    pub(super) fn is_integrated(self) -> bool {
        self == Self::Integrated
    }

    pub(super) fn mobile_page_script(self) -> &'static str {
        ["", Self::MOBILE_PAGE_SCRIPT][usize::from(self.is_integrated())]
    }
}
