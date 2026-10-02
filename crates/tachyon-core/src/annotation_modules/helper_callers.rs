use super::Module;
use crate::Failure;
use std::path::{Path, PathBuf};

pub(super) struct HelperCallers {
    entry: PathBuf,
    units: Vec<Module>,
}

impl HelperCallers {
    pub(super) fn check(
        entry: &Path,
        source: &str,
        modules: &[Module],
        environment: &str,
    ) -> Result<(), Failure> {
        Self::wanted(entry, environment)
            .then(|| Self::read(entry, source, modules).validate())
            .transpose()
            .map(|_| ())
    }

    fn wanted(entry: &Path, environment: &str) -> bool {
        (
            entry.extension().and_then(|value| value.to_str()),
            environment,
        ) == (Some("cs"), "tac")
    }

    fn read(entry: &Path, source: &str, modules: &[Module]) -> Self {
        let mut units = vec![Module {
            path: entry.to_path_buf(),
            source: source.to_owned(),
        }];
        units.extend(modules.iter().map(|module| Module {
            path: module.path.clone(),
            source: module.source.clone(),
        }));
        Self {
            entry: entry.to_path_buf(),
            units,
        }
    }

    fn validate(&self) -> Result<(), Failure> {
        let source = self
            .units
            .iter()
            .map(|unit| unit.source.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        crate::behavior::HelperRole::require_callers(&self.entry, &source)
            .map_err(|failure| self.located(failure))
    }

    fn located(&self, failure: Failure) -> Failure {
        let mut diagnostics = failure.diagnostics().to_vec();
        for diagnostic in &mut diagnostics {
            diagnostic.spans.iter_mut().for_each(|span| self.span(span));
        }
        Failure::new(diagnostics)
    }

    fn span(&self, span: &mut tachyon_diagnostics::SourceSpan) {
        let mut start = 0_u64;
        for unit in &self.units {
            self.unit_span(unit, start, span);
            start += unit.source.len() as u64 + 1;
        }
    }

    fn unit_span(&self, unit: &Module, start: u64, span: &mut tachyon_diagnostics::SourceSpan) {
        if span.file == self.entry.to_string_lossy()
            && (start..start + unit.source.len() as u64).contains(&span.start)
        {
            span.file = unit.path.to_string_lossy().into_owned();
            span.start -= start;
            span.end = span.end.saturating_sub(start).min(unit.source.len() as u64);
        }
    }
}
