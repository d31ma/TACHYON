use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "reach", rename_all = "camelCase")]
pub(super) enum TargetReach {
    Honored { how: String },
    RunOwed { how: String, run: String },
    Adapted { note: String },
    NotApplied { note: String },
    Refused { code: u16, alternative: String },
    NotApplicable { note: String },
}

impl TargetReach {
    pub(super) fn build_note(&self) -> Option<&str> {
        self.printed_note()
    }

    pub(super) fn owed_run(&self) -> Option<&str> {
        match self {
            Self::RunOwed { run, .. } => Some(run),
            Self::Honored { .. }
            | Self::Adapted { .. }
            | Self::NotApplied { .. }
            | Self::Refused { .. }
            | Self::NotApplicable { .. } => None,
        }
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

    #[cfg(test)]
    pub(super) fn is_honored(&self) -> bool {
        matches!(self, Self::Honored { .. })
    }

    fn printed_note(&self) -> Option<&str> {
        match self {
            Self::Adapted { note } | Self::NotApplied { note } | Self::NotApplicable { note } => {
                Some(note)
            }
            Self::Honored { .. } | Self::RunOwed { .. } | Self::Refused { .. } => None,
        }
    }

    fn refused_with(&self) -> Option<(u16, &str)> {
        match self {
            Self::Refused { code, alternative } => Some((*code, alternative)),
            Self::Honored { .. }
            | Self::RunOwed { .. }
            | Self::Adapted { .. }
            | Self::NotApplied { .. }
            | Self::NotApplicable { .. } => None,
        }
    }

    fn described(&self) -> String {
        match self {
            Self::Honored { how } => format!("honored: {how}"),
            Self::RunOwed { how, run } => format!("run owed: {how}; still to run: {run}"),
            Self::Adapted { note } => format!("adapted: {note}"),
            Self::NotApplied { note } => format!("not applied: {note}"),
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
            | Self::RunOwed { how: text, .. }
            | Self::Adapted { note: text }
            | Self::NotApplied { note: text }
            | Self::Refused {
                alternative: text, ..
            }
            | Self::NotApplicable { note: text } => text,
        }
    }
}
