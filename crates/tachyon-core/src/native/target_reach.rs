use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "reach", rename_all = "camelCase")]
pub(super) enum TargetReach {
    Honored { how: String },
    Adapted { note: String },
    Refused { code: u16, alternative: String },
    NotApplicable { note: String },
}

impl TargetReach {
    pub(super) fn build_note(&self) -> Option<&str> {
        self.printed_note()
    }

    pub(super) fn refusal(&self) -> Option<(u16, &str)> {
        self.refused_with()
    }

    pub(super) fn summary(&self) -> String {
        self.described()
    }

    #[cfg(test)]
    pub(super) fn text(&self) -> &str {
        self.own_text()
    }

    fn printed_note(&self) -> Option<&str> {
        match self {
            Self::Adapted { note } | Self::NotApplicable { note } => Some(note),
            Self::Honored { .. } | Self::Refused { .. } => None,
        }
    }

    fn refused_with(&self) -> Option<(u16, &str)> {
        match self {
            Self::Refused { code, alternative } => Some((*code, alternative)),
            Self::Honored { .. } | Self::Adapted { .. } | Self::NotApplicable { .. } => None,
        }
    }

    fn described(&self) -> String {
        match self {
            Self::Honored { how } => format!("honored: {how}"),
            Self::Adapted { note } => format!("adapted: {note}"),
            Self::Refused { code, alternative } => {
                format!("refused with TY{code}: {alternative}")
            }
            Self::NotApplicable { note } => format!("not applicable: {note}"),
        }
    }

    #[cfg(test)]
    fn own_text(&self) -> &str {
        match self {
            Self::Honored { how: text }
            | Self::Adapted { note: text }
            | Self::Refused {
                alternative: text, ..
            }
            | Self::NotApplicable { note: text } => text,
        }
    }
}
