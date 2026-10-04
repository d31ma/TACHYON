use serde::Deserialize;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct NativeTitleBarButtonInset {
    pub(super) leading: u32,
    pub(super) top: u32,
}

impl NativeTitleBarButtonInset {
    pub(super) const MAX_POINTS: u32 = 200;

    pub(super) fn is_within_bounds(self) -> bool {
        self.leading <= Self::MAX_POINTS && self.top <= Self::MAX_POINTS
    }
}
