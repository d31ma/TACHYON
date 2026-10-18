use serde::Deserialize;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(super) enum NativeTitleBar {
    #[default]
    Standard,
    Integrated,
}

impl NativeTitleBar {
    pub(super) fn is_integrated(self) -> bool {
        self == Self::Integrated
    }
}
