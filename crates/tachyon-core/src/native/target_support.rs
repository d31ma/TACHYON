use super::target_support_entry::TargetSupportEntry;
use crate::AppTarget;
use crate::failure::{Failure, diagnostic};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;
use tachyon_contracts::NativeTarget;
use tachyon_diagnostics::Diagnostic;

pub struct TargetSupport {
    entries: Vec<TargetSupportEntry>,
}

impl TargetSupport {
    const TABLE: &'static str = include_str!("target_support.json");
    const PLATFORMS: [&'static str; 6] = ["Web", "macOS", "Windows", "Linux", "iOS", "Android"];
    const DECLARED: [&'static str; 2] = ["", " (declared)"];

    /// Checks the members that `client/manifest.json` declares against one target.
    ///
    /// # Errors
    ///
    /// Returns the refusal of every declared member that the target refuses.
    pub fn notes(project_root: &Path, target: AppTarget) -> Result<Vec<String>, Failure> {
        Self::load()?.checked(&Self::manifest(project_root), target)
    }

    /// Describes what every manifest member does on each of the given targets.
    ///
    /// # Errors
    ///
    /// Returns a failure when the built-in support table cannot be read.
    pub fn explain(project_root: &Path, targets: &[AppTarget]) -> Result<String, Failure> {
        let table = Self::load()?;
        let manifest = Self::manifest(project_root);
        let sections: Vec<String> = targets
            .iter()
            .map(|target| table.target_text(*target, &manifest))
            .collect();
        Ok(sections.join("\n"))
    }

    /// Returns the note that one built-in capability has on one target.
    ///
    /// # Errors
    ///
    /// Returns a failure when the built-in support table cannot be read.
    pub fn capability_note(capability: &str, target: AppTarget) -> Result<Option<String>, Failure> {
        let table = Self::load()?;
        let note = table
            .entries
            .iter()
            .find(|entry| entry.member() == capability)
            .and_then(|entry| entry.reach(target).build_note());
        Ok(note.map(String::from))
    }

    pub(super) fn native_notes(
        project_root: &Path,
        target: NativeTarget,
    ) -> Result<Vec<String>, Failure> {
        Self::notes(project_root, Self::app_target(target))
    }

    pub(super) fn load() -> Result<Self, Failure> {
        serde_json::from_str(Self::TABLE)
            .map(|entries| Self { entries })
            .map_err(|error| Self::broken_table(&error))
    }

    #[cfg(test)]
    pub(super) fn entries(&self) -> &[TargetSupportEntry] {
        &self.entries
    }

    pub(super) fn platform(target: AppTarget) -> &'static str {
        Self::PLATFORMS[target as usize]
    }

    pub(super) fn target_text(&self, target: AppTarget, manifest: &Value) -> String {
        let lines: Vec<String> = self
            .entries
            .iter()
            .map(|entry| Self::entry_text(entry, target, manifest))
            .collect();
        format!("{}\n\n{}", Self::platform(target), lines.join("\n"))
    }

    fn entry_text(entry: &TargetSupportEntry, target: AppTarget, manifest: &Value) -> String {
        let declared = Self::DECLARED[usize::from(entry.is_declared_in(manifest))];
        format!(
            "{}{declared}\n  {}\n",
            entry.member(),
            entry.reach(target).summary()
        )
    }

    fn checked(&self, manifest: &Value, target: AppTarget) -> Result<Vec<String>, Failure> {
        self.refuse(manifest, target)?;
        Ok(self.printed_notes(manifest, target))
    }

    fn refuse(&self, manifest: &Value, target: AppTarget) -> Result<(), Failure> {
        let refusals: Vec<Diagnostic> = self
            .declared(manifest)
            .filter_map(|entry| Self::refusal(entry, target))
            .collect();
        if refusals.is_empty() {
            return Ok(());
        }
        Err(Failure::new(refusals))
    }

    fn printed_notes(&self, manifest: &Value, target: AppTarget) -> Vec<String> {
        let mut printed = BTreeSet::new();
        self.declared(manifest)
            .filter_map(|entry| Self::entry_note(entry, target))
            .filter(|note| printed.insert(note.clone()))
            .map(|note| format!("Note: {note}"))
            .collect()
    }

    fn entry_note(entry: &TargetSupportEntry, target: AppTarget) -> Option<String> {
        let reach = entry.reach(target);
        reach.build_note().map(String::from).or_else(|| {
            reach.owed_run().map(|run| {
                format!(
                    "{} is supported on {}, but this run is still owed: {run}.",
                    entry.member(),
                    Self::platform(target)
                )
            })
        })
    }

    fn declared<'a>(
        &'a self,
        manifest: &'a Value,
    ) -> impl Iterator<Item = &'a TargetSupportEntry> + 'a {
        self.entries
            .iter()
            .filter(move |entry| entry.is_declared_in(manifest))
    }

    fn refusal(entry: &TargetSupportEntry, target: AppTarget) -> Option<Diagnostic> {
        let platform = Self::platform(target);
        entry.reach(target).refusal().map(|(code, alternative)| {
            diagnostic(
                code,
                format!(
                    "client/manifest.json declares {}, which the {platform} target does not support.",
                    entry.member()
                ),
                Some(String::from(alternative)),
                None,
            )
        })
    }

    fn manifest(project_root: &Path) -> Value {
        std::fs::read_to_string(project_root.join(super::config::WebAppManifest::SOURCE_PATH))
            .ok()
            .and_then(|source| serde_json::from_str(crate::without_bom(&source)).ok())
            .unwrap_or(Value::Null)
    }

    pub(super) fn native_platform(target: NativeTarget) -> &'static str {
        Self::platform(Self::app_target(target))
    }

    fn app_target(target: NativeTarget) -> AppTarget {
        match target {
            NativeTarget::Macos => AppTarget::Macos,
            NativeTarget::Windows => AppTarget::Windows,
            NativeTarget::Linux => AppTarget::Linux,
            NativeTarget::Ios => AppTarget::Ios,
            NativeTarget::Android => AppTarget::Android,
        }
    }

    fn broken_table(error: &serde_json::Error) -> Failure {
        Failure::one(diagnostic(
            1605,
            format!("The built-in per-target support table is invalid: {error}"),
            None,
            None,
        ))
    }
}
