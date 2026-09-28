use super::edits::{Hunk, TextEdit};
use super::inline_model::InlineModel;
use super::inline_models::InlineModels;
use super::known_model::KnownModel;
use super::realm_models::RealmModels;
use super::{FileChange, ManualStep};
use crate::annotation_layout::InlineModelCheck;
use crate::route_types::{Model, Shape};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct InlineModelFix {
    pub(super) changes: Vec<FileChange>,
    pub(super) manual_steps: Vec<ManualStep>,
    models: RealmModels,
}

impl InlineModelFix {
    const NOTE: &'static str =
        "A JavaScript model is one @typedef in its own file in the models folder (ADR 0067).";

    pub(super) fn plan(sources: &[(String, String)], taken: &[String]) -> Self {
        let mut fix = Self {
            changes: Vec::new(),
            manual_steps: Vec::new(),
            models: RealmModels::read(sources),
        };
        fix.add_files(&Self::files_with_models(sources, taken));
        fix
    }

    pub(super) fn created_files(&self) -> Vec<String> {
        self.changes
            .iter()
            .filter(|change| change.original.is_empty())
            .map(|change| change.source.to_lowercase())
            .collect()
    }

    fn files_with_models<'s>(
        sources: &'s [(String, String)],
        taken: &[String],
    ) -> Vec<InlineModels<'s>> {
        let mut open: Vec<&(String, String)> = sources
            .iter()
            .filter(|(relative, _)| Self::is_open(relative, taken))
            .collect();
        open.sort();
        open.into_iter()
            .filter_map(|(relative, source)| InlineModels::read(relative, source))
            .collect()
    }

    fn is_open(relative: &String, taken: &[String]) -> bool {
        let checks = [
            !taken.contains(relative),
            InlineModelCheck::applies(Path::new(relative)),
        ];
        !checks.contains(&false)
    }

    fn add_files(&mut self, files: &[InlineModels<'_>]) {
        for file in files {
            self.add_file(file);
        }
    }

    fn add_file(&mut self, file: &InlineModels<'_>) {
        let conflicts = self.conflicts(file);
        if !conflicts.is_empty() {
            self.explain(file, &conflicts);
            return;
        }
        let edits: Vec<TextEdit> = file
            .models
            .iter()
            .map(|model| self.resolved(file, model))
            .collect();
        self.add_change(file, &edits);
    }

    fn conflicts(&self, file: &InlineModels<'_>) -> Vec<String> {
        file.models
            .iter()
            .filter_map(|model| self.conflict(file, model))
            .collect()
    }

    fn conflict(&self, file: &InlineModels<'_>, model: &InlineModel) -> Option<String> {
        let name = &model.block.name;
        let known = self.models.find(file.realm, name)?;
        let differs = !known.is_same(&model.model);
        differs.then(|| format!("'{name}' differs from {}", known.described))
    }

    fn explain(&mut self, file: &InlineModels<'_>, conflicts: &[String]) {
        self.manual_steps.push(ManualStep::new(
            file.relative,
            format!(
                "This file declares models with @typedef, and {}. {}",
                conflicts.join("; "),
                Self::NOTE
            ),
            format!(
                "Make each @typedef match the model it copies, or give it another name, then run 'ty migrate fix --write' again to move it to '{}/models/'.",
                file.realm
            ),
        ));
    }

    fn resolved(&mut self, file: &InlineModels<'_>, model: &InlineModel) -> TextEdit {
        let replacement = self.replacement(file, model);
        TextEdit::replace(model.block.start, file.removal_end(model), &replacement)
    }

    fn replacement(&mut self, file: &InlineModels<'_>, model: &InlineModel) -> String {
        let name = &model.block.name;
        let Some(known) = self.models.find(file.realm, name) else {
            self.create(file, model);
            return String::new();
        };
        known.import_line(file.realm, name)
    }

    fn create(&mut self, file: &InlineModels<'_>, model: &InlineModel) {
        let name = &model.block.name;
        let target = format!("{}/models/{name}.js", file.realm);
        let imports = self.referenced_imports(file.realm, &model.model);
        let text = format!("{imports}{}\nexport {{}}\n", file.block_text(model).trim_end());
        let known = KnownModel::planned(&target, file.relative, model.model.clone());
        self.models.add(file.realm, name, known);
        let edits = [TextEdit::insert(0, &text)];
        self.changes.push(FileChange {
            notes: vec![format!(
                "New model file for {name}, moved from '{}'. {}",
                file.relative,
                Self::NOTE
            )],
            hunks: Hunk::from_edits("", &edits),
            source: target,
            original: String::new(),
            updated: text,
        });
    }

    fn referenced_imports(&self, realm: &str, model: &Model) -> String {
        Self::model_shapes(model)
            .into_iter()
            .flat_map(Self::shape_names)
            .collect::<BTreeSet<String>>()
            .into_iter()
            .filter_map(|name| self.import_for(realm, &name))
            .collect()
    }

    fn import_for(&self, realm: &str, name: &str) -> Option<String> {
        let known = self.models.find(realm, name)?;
        let line = known.import_line(realm, name);
        (!line.is_empty()).then_some(line)
    }

    fn model_shapes(model: &Model) -> Vec<&Shape> {
        match model {
            Model::Alias(shape) => vec![shape],
            Model::Record(record) => record.fields.iter().map(|field| &field.shape).collect(),
        }
    }

    fn shape_names(shape: &Shape) -> BTreeSet<String> {
        match shape {
            Shape::Model(name) => BTreeSet::from([name.clone()]),
            Shape::List(inner) | Shape::Optional(inner) => Self::shape_names(inner),
            _ => BTreeSet::new(),
        }
    }

    fn add_change(&mut self, file: &InlineModels<'_>, edits: &[TextEdit]) {
        let names: Vec<&str> = file
            .models
            .iter()
            .map(|model| model.block.name.as_str())
            .collect();
        self.changes.push(FileChange {
            source: file.relative.to_owned(),
            notes: vec![format!(
                "Moved the @typedef of {} to the models folder. {}",
                names.join(", "),
                Self::NOTE
            )],
            hunks: Hunk::from_edits(file.source, edits),
            original: file.source.to_owned(),
            updated: TextEdit::apply(file.source, edits),
        });
    }
}
