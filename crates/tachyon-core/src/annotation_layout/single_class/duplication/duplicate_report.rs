use super::project_source::ProjectSource;
use super::utility_name::UtilityName;
use crate::failure::{diagnostic, source_span};
use crate::migrate::{MigrationFinding, MigrationStatus};
use tachyon_diagnostics::Diagnostic;
use tachyon_duplication::Duplicate;

pub(super) struct DuplicateReport<'d> {
    duplicate: &'d Duplicate,
    source: Option<&'d ProjectSource>,
}

impl<'d> DuplicateReport<'d> {
    const CODE: u16 = 1315;
    const FEATURE: &'static str = "duplication.block";
    const TAC_OPT_OUT: &'static str = "`tac.duplication.check` in client/manifest.json";
    const YON_OPT_OUT: &'static str = "`duplication.check` in server/application.json";

    pub(super) fn new(duplicate: &'d Duplicate, sources: &'d [ProjectSource]) -> Self {
        let source = sources
            .iter()
            .find(|source| source.path == duplicate.second.path);
        Self { duplicate, source }
    }

    pub(super) fn diagnostic(&self) -> Diagnostic {
        let (start, end) = self.byte_range();
        let span = source_span(&self.duplicate.second.path, start, end);
        diagnostic(Self::CODE, self.message(), Some(self.help()), span)
    }

    pub(super) fn finding(&self) -> MigrationFinding {
        MigrationFinding {
            source: self.duplicate.second.path.clone(),
            feature: Self::FEATURE.to_owned(),
            status: MigrationStatus::Changed,
            detail: self.message(),
            action: Some(self.help()),
        }
    }

    fn message(&self) -> String {
        let second = &self.duplicate.second;
        format!(
            "Duplicated code: lines {}-{} repeat {} ({} tokens; only names and values may differ).",
            second.first_line,
            second.last_line,
            self.duplicate.first.label(),
            self.duplicate.tokens
        )
    }

    fn help(&self) -> String {
        let name = UtilityName::suggest(&self.duplicate.first.path, &self.duplicate.second.path);
        let realm = self.source.map_or("server", ProjectSource::realm);
        let extension = self.source.map_or("", ProjectSource::extension);
        let client = usize::from(realm == "client");
        let opt_out = [Self::YON_OPT_OUT, Self::TAC_OPT_OUT][client];
        let folder = ["server/utils", crate::annotation_layout::UtilityLayout::CLIENT_ROOT][client];
        format!(
            "Move the repeated code into a utility: create {folder}/{name}Utils.{extension} with a static method, and call it from both places. While you migrate, you can set {opt_out} to false (ADR 0064)."
        )
    }

    fn byte_range(&self) -> (usize, usize) {
        let text = self.source.map_or("", |source| source.text.as_str());
        let second = &self.duplicate.second;
        let start = Self::line_offset(text, second.first_line);
        let end = Self::line_offset(text, second.last_line + 1).max(start);
        (start, end)
    }

    fn line_offset(text: &str, line: usize) -> usize {
        text.split_inclusive('\n')
            .take(line.saturating_sub(1))
            .map(str::len)
            .sum()
    }
}
