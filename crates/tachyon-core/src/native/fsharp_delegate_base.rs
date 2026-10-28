use std::ops::Range;

pub(super) struct FsharpDelegateBase {
    pub(super) delegate: String,
    pub(super) kind: String,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) marker: Range<usize>,
    pub(super) value: Range<usize>,
}

impl FsharpDelegateBase {
    const NEED: &'static str = "TacDelegateBase.Need<{kind}>({instance})";

    pub(super) fn need(&self, instance: &str) -> String {
        Self::NEED
            .replace("{kind}", &self.kind)
            .replace("{instance}", &format!("{instance:?}"))
    }
}
