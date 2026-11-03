#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AttributeState {
    Missing,
    Blank,
    Filled,
}

impl AttributeState {
    pub(super) fn of(static_text: Option<&str>) -> Self {
        static_text.map_or(Self::Filled, Self::of_text)
    }

    fn of_text(text: &str) -> Self {
        if text.trim().is_empty() {
            Self::Blank
        } else {
            Self::Filled
        }
    }
}
