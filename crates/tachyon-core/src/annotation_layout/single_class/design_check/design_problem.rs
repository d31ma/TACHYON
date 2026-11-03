use super::design_rule::DesignRule;
use crate::failure::{diagnostic, source_span};
use crate::migrate::{MigrationFinding, MigrationStatus};
use tachyon_diagnostics::Diagnostic;

pub(super) struct DesignProblem {
    pub(super) rule: DesignRule,
    pub(super) path: String,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) line: usize,
    pub(super) message: String,
    pub(super) help: String,
}

impl DesignProblem {
    const CLIENT_PREFIX: &'static str = "client/";

    pub(super) fn diagnostic(&self) -> Diagnostic {
        let span = source_span(&self.path, self.start, self.end);
        diagnostic(
            self.rule.code(),
            &self.message,
            Some(self.full_help()),
            span,
        )
    }

    pub(super) fn finding(&self) -> MigrationFinding {
        MigrationFinding {
            source: self.path.clone(),
            feature: self.rule.feature().to_owned(),
            status: MigrationStatus::Changed,
            detail: format!("Line {}: {}", self.line, self.message),
            action: Some(self.help.clone()),
        }
    }

    fn full_help(&self) -> String {
        format!("{} {}", self.help, self.opt_out())
    }

    fn opt_out(&self) -> String {
        let name = self.rule.switch_name();
        let client = self.path.starts_with(Self::CLIENT_PREFIX);
        let options = [
            format!("`{name}.check` in server/application.json"),
            format!("`tac.{name}.check` in client/manifest.json"),
        ];
        format!(
            "While you migrate, you can set {} to false (ADR 0069).",
            options[usize::from(client)]
        )
    }
}
