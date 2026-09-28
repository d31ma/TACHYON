#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Consent {
    Always,
    Ask,
    Never,
}

impl Consent {
    #[must_use]
    pub fn from_options(answer_yes: bool, can_ask: bool) -> Self {
        Self::chosen(answer_yes, can_ask)
    }

    fn chosen(answer_yes: bool, can_ask: bool) -> Self {
        if answer_yes {
            Self::Always
        } else if can_ask {
            Self::Ask
        } else {
            Self::Never
        }
    }
}
