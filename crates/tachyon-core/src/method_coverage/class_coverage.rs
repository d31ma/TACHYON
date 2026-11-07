use super::{CallHooks, CoverageEvidence, CoveredMethod, MethodState, RoleClass, Uncovered};
use crate::Failure;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassCoverage {
    pub file: String,
    pub type_name: String,
    pub language: String,
    pub recorded: bool,
    pub methods: Vec<CoveredMethod>,
}

impl ClassCoverage {
    pub(crate) fn read(
        relative: &Path,
        source: &str,
        evidence: &CoverageEvidence,
    ) -> Result<Option<Self>, Failure> {
        let role = RoleClass::read(relative, source)?;
        Ok(role.map(|role| Self::judge(&role, evidence)))
    }

    #[must_use]
    pub fn uncovered(&self) -> Vec<Uncovered> {
        self.methods
            .iter()
            .filter(|method| !method.state.is_covered())
            .map(|method| self.missing(method))
            .collect()
    }

    fn missing(&self, method: &CoveredMethod) -> Uncovered {
        Uncovered {
            file: self.file.clone(),
            type_name: self.type_name.clone(),
            method: method.name.clone(),
            recorded: self.recorded,
        }
    }

    fn judge(role: &RoleClass, evidence: &CoverageEvidence) -> Self {
        let recorded = Self::is_recorded(role, evidence);
        Self {
            file: role.file.clone(),
            type_name: role.type_name.clone(),
            language: role.language.clone(),
            recorded,
            methods: Self::judge_methods(role, recorded, evidence),
        }
    }

    fn is_recorded(role: &RoleClass, evidence: &CoverageEvidence) -> bool {
        CallHooks::records(&role.realm, &role.language)
            && !evidence.calls.could_not_run(&role.realm, &role.language)
    }

    fn judge_methods(
        role: &RoleClass,
        recorded: bool,
        evidence: &CoverageEvidence,
    ) -> Vec<CoveredMethod> {
        if recorded {
            return Self::methods_by_calls(role, evidence);
        }
        Self::methods_by_names(role, evidence)
    }

    fn methods_by_calls(role: &RoleClass, evidence: &CoverageEvidence) -> Vec<CoveredMethod> {
        role.methods
            .iter()
            .map(|method| Self::covered(&method.name, Self::called(role, &method.name, evidence)))
            .collect()
    }

    fn methods_by_names(role: &RoleClass, evidence: &CoverageEvidence) -> Vec<CoveredMethod> {
        role.methods
            .iter()
            .map(|method| Self::covered(&method.name, Self::named(role, &method.name, evidence)))
            .collect()
    }

    fn covered(name: &str, state: MethodState) -> CoveredMethod {
        CoveredMethod {
            name: name.to_owned(),
            state,
        }
    }

    fn called(role: &RoleClass, name: &str, evidence: &CoverageEvidence) -> MethodState {
        evidence
            .calls
            .first_case(&role.realm, &role.key(name))
            .map_or(MethodState::Missing, |case| {
                MethodState::Called(case.to_owned())
            })
    }

    fn named(role: &RoleClass, name: &str, evidence: &CoverageEvidence) -> MethodState {
        if evidence.vocabularies.names(&role.type_name, name) {
            return MethodState::Named;
        }
        MethodState::Missing
    }
}
