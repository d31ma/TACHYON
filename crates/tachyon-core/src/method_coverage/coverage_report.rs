use super::{
    ClassCoverage, CoverageEvidence, MethodState, RecordedCalls, RoleFiles, Uncovered,
    UnitVocabularies,
};
use crate::Failure;
use std::path::Path;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CoverageReport {
    pub classes: Vec<ClassCoverage>,
}

impl CoverageReport {
    pub fn build(project: &Path, calls: RecordedCalls) -> Result<Self, Failure> {
        let evidence = CoverageEvidence {
            calls,
            vocabularies: UnitVocabularies::read(project),
        };
        let classes = Self::classes(project, &evidence)?;
        Ok(Self { classes })
    }

    #[must_use]
    pub fn covered(&self) -> usize {
        self.states().filter(|state| state.is_covered()).count()
    }

    #[must_use]
    pub fn total(&self) -> usize {
        self.states().count()
    }

    #[must_use]
    pub fn uncovered(&self) -> Vec<Uncovered> {
        self.classes
            .iter()
            .flat_map(ClassCoverage::uncovered)
            .collect()
    }

    fn states(&self) -> impl Iterator<Item = &MethodState> {
        self.classes
            .iter()
            .flat_map(|class| class.methods.iter())
            .map(|method| &method.state)
    }

    fn classes(project: &Path, evidence: &CoverageEvidence) -> Result<Vec<ClassCoverage>, Failure> {
        let mut classes = Vec::new();
        for (relative, source) in RoleFiles::sources(project) {
            classes.extend(ClassCoverage::read(&relative, &source, evidence)?);
        }
        Ok(classes)
    }
}
