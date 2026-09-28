use crate::failure::{diagnostic, source_span};
use crate::template::model::SourceRange;
use tachyon_diagnostics::Diagnostic;

pub(super) struct AccessibilityProblem {
    range: SourceRange,
    message: String,
    help: &'static str,
}

impl AccessibilityProblem {
    pub(super) const CODE: u16 = 1336;
    const SWITCH_HELP: &'static str = "While a project migrates, `tac.accessibility.check: false` in client/manifest.json turns this rule off (ADR 0074).";

    pub(super) fn new(range: SourceRange, message: String, help: &'static str) -> Self {
        Self {
            range,
            message,
            help,
        }
    }

    pub(super) fn diagnostic(&self, source_path: &str) -> Diagnostic {
        let help = format!("{} {}", self.help, Self::SWITCH_HELP);
        let span = source_span(source_path, self.range.start, self.range.end);
        diagnostic(Self::CODE, self.message.clone(), Some(help), span)
    }
}
