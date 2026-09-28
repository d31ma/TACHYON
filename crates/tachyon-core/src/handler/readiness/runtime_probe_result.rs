use super::{RuntimeProbeState, RuntimeRequirement};
use crate::Failure;
use crate::failure::diagnostic;
use tachyon_diagnostics::Diagnostic;

#[derive(Clone)]
pub(crate) struct RuntimeProbeResult {
    requirement: RuntimeRequirement,
    state: RuntimeProbeState,
}

impl RuntimeProbeResult {
    pub(super) fn new(requirement: RuntimeRequirement, state: RuntimeProbeState) -> Self {
        Self { requirement, state }
    }

    pub(crate) fn requirement(&self) -> &RuntimeRequirement {
        &self.requirement
    }

    pub(crate) const fn state(&self) -> RuntimeProbeState {
        self.state
    }

    pub(crate) fn failure(results: &[Self]) -> Option<Failure> {
        let diagnostics: Vec<Diagnostic> = results.iter().filter_map(Self::diagnostic).collect();
        Self::failure_from(diagnostics)
    }

    fn diagnostic(&self) -> Option<Diagnostic> {
        let label = self.requirement.label();
        let help = self.requirement.help();
        match self.state {
            RuntimeProbeState::Ready => None,
            RuntimeProbeState::Missing => Some(diagnostic(
                2112,
                format!("Required {label} was not found."),
                Some(help.to_owned()),
                None,
            )),
            RuntimeProbeState::Incomplete => Some(diagnostic(
                2101,
                format!("Required {label} is not usable."),
                Some(help.to_owned()),
                None,
            )),
        }
    }

    fn failure_from(diagnostics: Vec<Diagnostic>) -> Option<Failure> {
        if diagnostics.is_empty() {
            return None;
        }
        Some(Failure::new(diagnostics))
    }
}
