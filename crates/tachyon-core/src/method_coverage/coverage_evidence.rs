use super::{RecordedCalls, UnitVocabularies};

pub(crate) struct CoverageEvidence {
    pub(crate) calls: RecordedCalls,
    pub(crate) vocabularies: UnitVocabularies,
}
