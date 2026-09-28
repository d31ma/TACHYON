use super::edits::{Hunk, TextEdit};
use super::model_importers::ModelImporters;
use super::model_split::ModelSplit;
use super::{FileChange, ManualStep};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct ModelSplitFix {
    pub(super) changes: Vec<FileChange>,
    pub(super) manual_steps: Vec<ManualStep>,
    planned_files: Vec<String>,
    importer_edits: BTreeMap<String, Vec<TextEdit>>,
}

impl ModelSplitFix {
    const NOTE: &'static str = "A models file holds one model (ADR 0067).";

    pub(super) fn plan(sources: &[(String, String)], taken: &[String]) -> Self {
        let mut fix = Self::default();
        sources
            .iter()
            .filter(|(relative, _)| !taken.contains(relative))
            .filter_map(|(relative, source)| ModelSplit::read(relative, source))
            .for_each(|split| fix.add_split(sources, &split));
        fix.add_importer_changes(sources, taken);
        fix
    }

    pub(super) fn created_files(&self) -> Vec<String> {
        self.changes
            .iter()
            .filter(|change| change.original.is_empty())
            .map(|change| change.source.to_lowercase())
            .collect()
    }

    fn add_split(&mut self, sources: &[(String, String)], split: &ModelSplit<'_>) {
        let taken_target = self.taken_target(sources, split);
        if let Some(target) = taken_target {
            self.explain_taken(split, &target);
            return;
        }
        self.add_original(split);
        self.add_new_files(split);
        self.collect_importer_edits(sources, split);
    }

    fn taken_target(&self, sources: &[(String, String)], split: &ModelSplit<'_>) -> Option<String> {
        let existing: Vec<String> = sources
            .iter()
            .map(|(relative, _)| relative.to_lowercase())
            .chain(self.planned_files.iter().cloned())
            .collect();
        split
            .moved_names()
            .into_iter()
            .map(|name| split.target(name))
            .find(|target| existing.contains(&target.to_lowercase()))
    }

    fn explain_taken(&mut self, split: &ModelSplit<'_>, target: &str) {
        self.manual_steps.push(ManualStep::new(
            split.relative,
            format!("This models file declares more than one model, and '{target}' already exists. {}", Self::NOTE),
            format!("Move each extra model of '{}' into its own file named after the model, and choose another name for the model that clashes with '{target}'.", split.relative),
        ));
    }

    fn add_original(&mut self, split: &ModelSplit<'_>) {
        let original = split.source_text();
        let edits = split.original_edits();
        self.changes.push(FileChange {
            source: split.relative.to_owned(),
            notes: vec![format!(
                "Moved {} into its own file. {}",
                split.moved_names().join(", "),
                Self::NOTE
            )],
            hunks: Hunk::from_edits(original, &edits),
            original: original.to_owned(),
            updated: TextEdit::apply(original, &edits),
        });
    }

    fn add_new_files(&mut self, split: &ModelSplit<'_>) {
        for block in split.moved() {
            let target = split.target(&block.name);
            let text = split.new_file(block);
            let edits = [TextEdit::insert(0, &text)];
            self.planned_files.push(target.to_lowercase());
            self.changes.push(FileChange {
                source: target,
                notes: vec![format!(
                    "New file for {}, split from '{}'. {}",
                    block.name,
                    split.relative,
                    Self::NOTE
                )],
                hunks: Hunk::from_edits("", &edits),
                original: String::new(),
                updated: text,
            });
        }
    }

    fn collect_importer_edits(&mut self, sources: &[(String, String)], split: &ModelSplit<'_>) {
        let importers = ModelImporters::new(split);
        let read = sources
            .iter()
            .filter(|(relative, _)| importers.reads(relative));
        for (relative, source) in read {
            let edits = importers.edits(relative, source);
            self.importer_edits
                .entry(relative.clone())
                .or_default()
                .extend(edits);
        }
    }

    fn add_importer_changes(&mut self, sources: &[(String, String)], taken: &[String]) {
        let edited: Vec<(String, Vec<TextEdit>)> = std::mem::take(&mut self.importer_edits)
            .into_iter()
            .filter(|(_, edits)| !edits.is_empty())
            .collect();
        for (relative, edits) in edited {
            let source = Self::source_of(sources, &relative);
            self.add_importer(&relative, source, &edits, taken);
        }
    }

    fn source_of<'s>(sources: &'s [(String, String)], relative: &str) -> &'s str {
        sources
            .iter()
            .find(|(path, _)| path == relative)
            .map_or("", |(_, source)| source.as_str())
    }

    fn add_importer(&mut self, relative: &str, source: &str, edits: &[TextEdit], taken: &[String]) {
        let busy = [
            taken.contains(&relative.to_owned()),
            self.changes.iter().any(|change| change.source == relative),
            !TextEdit::are_separate(edits),
        ];
        if busy.contains(&true) {
            self.explain_importer(relative);
            return;
        }
        self.changes.push(FileChange {
            source: relative.to_owned(),
            notes: vec![format!(
                "Imports each moved model from its own file. {}",
                Self::NOTE
            )],
            hunks: Hunk::from_edits(source, edits),
            original: source.to_owned(),
            updated: TextEdit::apply(source, edits),
        });
    }

    fn explain_importer(&mut self, relative: &str) {
        self.manual_steps.push(ManualStep::new(
            relative,
            format!(
                "This file imports a model that the fix moved into its own file. {}",
                Self::NOTE
            ),
            String::from("Import each moved model from the file that is named after it."),
        ));
    }
}
