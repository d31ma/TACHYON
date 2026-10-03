use super::converters::Declarations;
use super::model_files::ModelParser;
use super::{Model, ModelTable, RustField, WebEventModels};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct RustTacModels {
    files: Vec<(String, String)>,
}

impl RustTacModels {
    const LANGUAGE: &'static str = "rs";
    const MASK_PATH: &'static str = "model.rs";
    const SCALARS: [&'static str; 8] = [
        "String",
        "std::string::String",
        "::std::string::String",
        "i64",
        "i32",
        "f64",
        "bool",
        "TacValue",
    ];
    const PRIMITIVE_PREFIXES: [&'static str; 4] = [
        "std::primitive::",
        "::std::primitive::",
        "core::primitive::",
        "::core::primitive::",
    ];
    const PRIMITIVES: [&'static str; 4] = ["i64", "i32", "f64", "bool"];
    const CONTAINERS: [&'static str; 8] = [
        "Vec",
        "std::vec::Vec",
        "::std::vec::Vec",
        "Option",
        "std::option::Option",
        "::std::option::Option",
        "core::option::Option",
        "::core::option::Option",
    ];
    const MODEL_PREFIXES: [&'static str; 2] = ["crate::models::", "::crate::models::"];
    const MAX_MODELS: usize = 256;
    const MAX_TYPE_DEPTH: usize = 32;
    const VISIBILITY: [&'static str; 3] = ["pub ", "pub(crate) ", "pub(super) "];

    pub(super) fn of(files: &[(String, String)]) -> Self {
        Self {
            files: files.to_vec(),
        }
    }

    pub(super) fn text(&self) -> String {
        self.generated()
    }

    pub(super) fn require(&self, kinds: &[String], path: &Path) -> Result<(), Failure> {
        self.require_roots(kinds, path)
    }

    fn require_roots(&self, kinds: &[String], path: &Path) -> Result<(), Failure> {
        kinds.iter().try_for_each(|kind| {
            self.check_type(kind, &mut BTreeSet::new())
                .map_err(|failure| Self::located(failure, path))
        })
    }

    fn located(failure: Failure, path: &Path) -> Failure {
        if failure
            .diagnostics()
            .iter()
            .any(|item| !item.spans.is_empty())
        {
            return failure;
        }
        Self::failure(path, 0..1, &failure.to_string())
    }

    fn generated(&self) -> String {
        self.files
            .iter()
            .flat_map(|(_, source)| self.file_text(source))
            .collect()
    }

    fn file_text(&self, source: &str) -> Vec<String> {
        ModelParser::models(Self::LANGUAGE, source)
            .into_iter()
            .filter_map(|(name, model)| self.model_text(source, &name, &model))
            .collect()
    }

    fn model_text(&self, source: &str, name: &str, model: &Model) -> Option<String> {
        match model {
            Model::Record(_) => self.record_text(name),
            Model::Alias(_) => Self::enumeration(source, name),
        }
    }

    fn record_text(&self, name: &str) -> Option<String> {
        self.check_type(name, &mut BTreeSet::new()).ok()?;
        let (_, fields) = self.fields(name)?;
        Some(Self::record(name, &fields))
    }

    fn fields(&self, name: &str) -> Option<(&str, Vec<RustField>)> {
        self.files.iter().find_map(|(path, source)| {
            ModelTable::rust_model_fields(source)
                .into_iter()
                .find(|(declared, _)| declared == name)
                .map(|(_, fields)| (path.as_str(), fields))
        })
    }

    fn check_type(&self, kind: &str, seen: &mut BTreeSet<String>) -> Result<(), Failure> {
        let compact = Self::compact_type(kind)?;
        Self::scalar(&compact)
            .then_some(Ok(()))
            .unwrap_or_else(|| self.check_container(&compact, seen))
    }

    fn scalar(kind: &str) -> bool {
        Self::SCALARS.contains(&kind)
            || Self::PRIMITIVE_PREFIXES
                .iter()
                .filter_map(|prefix| kind.strip_prefix(prefix))
                .any(|primitive| Self::PRIMITIVES.contains(&primitive))
    }

    fn compact_type(kind: &str) -> Result<String, Failure> {
        if kind.bytes().filter(|byte| *byte == b'<').count() > Self::MAX_TYPE_DEPTH {
            return Err(Self::unsupported(kind));
        }
        Ok(kind
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect())
    }

    fn check_container(&self, kind: &str, seen: &mut BTreeSet<String>) -> Result<(), Failure> {
        match Self::item(kind) {
            Some(item) => self.check_type(item, seen),
            None => self.check_named(kind, seen),
        }
    }

    fn item(kind: &str) -> Option<&str> {
        let (base, rest) = kind.split_once('<')?;
        Self::CONTAINERS.contains(&base).then_some(())?;
        rest.strip_suffix('>')
    }

    fn check_named(&self, kind: &str, seen: &mut BTreeSet<String>) -> Result<(), Failure> {
        let name = Self::MODEL_PREFIXES
            .iter()
            .find_map(|prefix| kind.strip_prefix(prefix))
            .unwrap_or(kind);
        self.provided(name)
            .then_some(Ok(()))
            .unwrap_or_else(|| self.check_alias_or_record(name, seen))
    }

    fn provided(&self, name: &str) -> bool {
        WebEventModels::names()
            .iter()
            .any(|provided| provided == name)
            || self
                .files
                .iter()
                .any(|(_, source)| Self::variants(source, name).is_some())
    }

    fn check_alias_or_record(
        &self,
        name: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        let target = self
            .files
            .iter()
            .find_map(|(_, source)| Self::alias(source, name));
        match target {
            Some(target) => self.check_alias(name, &target, seen),
            None => self.check_record(name, seen),
        }
    }

    fn check_alias(
        &self,
        name: &str,
        target: &str,
        seen: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        Self::limit(seen)?;
        Self::enter_alias(name, seen)?;
        let result = self.check_type(target, seen);
        seen.remove(name);
        result
    }

    fn enter_alias(name: &str, seen: &mut BTreeSet<String>) -> Result<(), Failure> {
        if !seen.insert(name.to_owned()) {
            return Err(Self::unsupported(name));
        }
        Ok(())
    }

    fn alias(source: &str, name: &str) -> Option<String> {
        let code =
            crate::annotation_layout::masked_source(Path::new(Self::MASK_PATH), source).ok()?;
        code.lines().find_map(|line| Self::alias_line(line, name))
    }

    fn alias_line(line: &str, name: &str) -> Option<String> {
        let line = line.trim();
        let line = Self::VISIBILITY
            .iter()
            .find_map(|prefix| line.strip_prefix(prefix))
            .unwrap_or(line);
        let (declared, target) = line.strip_prefix("type ")?.split_once('=')?;
        (declared.trim() == name).then(|| target.trim().trim_end_matches(';').trim().to_owned())
    }

    fn check_record(&self, name: &str, seen: &mut BTreeSet<String>) -> Result<(), Failure> {
        let (path, fields) = self.fields(name).ok_or_else(|| Self::unsupported(name))?;
        Self::limit(seen)?;
        seen.insert(name.to_owned())
            .then_some(())
            .map(|()| self.check_fields(path, &fields, seen))
            .transpose()
            .map(|_| ())
    }

    fn limit(seen: &BTreeSet<String>) -> Result<(), Failure> {
        if seen.len() >= Self::MAX_MODELS {
            return Err(Self::unsupported("model closure exceeds 256 declarations"));
        }
        Ok(())
    }

    fn check_fields(
        &self,
        path: &str,
        fields: &[RustField],
        seen: &mut BTreeSet<String>,
    ) -> Result<(), Failure> {
        fields.iter().try_for_each(|field| {
            self.check_type(&field.type_text, seen)
                .map_err(|failure| Self::at_field(failure, path, field))
        })
    }

    fn at_field(failure: Failure, path: &str, field: &RustField) -> Failure {
        let message = format!(
            "Rust Tac model field '{}' has unsupported wire type '{}'. Other generic or path fields may stay inside Rust; they cannot cross a page member, event or typed Convert boundary. {failure}",
            field.name, field.type_text
        );
        Self::located_field(failure, path, field, &message)
    }

    fn located_field(failure: Failure, path: &str, field: &RustField, message: &str) -> Failure {
        if failure
            .diagnostics()
            .iter()
            .any(|item| !item.spans.is_empty())
        {
            return failure;
        }
        let mut item = diagnostic(
            1405,
            message,
            None,
            source_span(path, field.type_range.start, field.type_range.end),
        );
        item.spans.extend(source_span(
            path,
            field.name_range.start,
            field.name_range.end,
        ));
        Failure::one(item)
    }

    fn unsupported(kind: &str) -> Failure {
        Failure::one(diagnostic(
            1405,
            format!(
                "Rust Tac wire type '{kind}' requires a supported primitive, Vec/Option, or registered codec-eligible model (ADR0035). Map/set/tuple wire serialization is not supported."
            ),
            None,
            None,
        ))
    }

    fn failure(path: &Path, range: std::ops::Range<usize>, message: &str) -> Failure {
        Failure::one(diagnostic(
            1405,
            message,
            None,
            source_span(&path.to_string_lossy(), range.start, range.end),
        ))
    }

    fn record(name: &str, fields: &[RustField]) -> String {
        let reads = fields
            .iter()
            .map(|field| format!("{}: tac_read_field(value, {:?})", field.name, field.name))
            .collect::<Vec<_>>();
        let writes = fields
            .iter()
            .map(|field| {
                format!(
                    "(String::from({:?}), TacModel::write(self.{}))",
                    field.name, field.name
                )
            })
            .collect::<Vec<_>>();
        format!(
            "\nimpl TacModel for crate::models::{name} {{\n    fn read(value: &TacValue) -> Self {{\n        Self {{ {} }}\n    }}\n    fn write(self) -> TacValue {{\n        TacValue::Object(vec![{}])\n    }}\n}}\n",
            reads.join(", "),
            writes.join(", ")
        )
    }

    fn variants(source: &str, name: &str) -> Option<Vec<String>> {
        let code =
            crate::annotation_layout::masked_source(Path::new(Self::MASK_PATH), source).ok()?;
        let header = format!("enum {name}");
        let (at, _) = code
            .match_indices(&header)
            .find(|(at, _)| code[at + header.len()..].trim_start().starts_with('{'))?;
        Declarations::variants(&code[at..], name)
    }

    fn enumeration(source: &str, name: &str) -> Option<String> {
        let variants = Self::variants(source, name)?;
        let first = variants.first()?;
        let mut reads = String::new();
        let mut writes = String::new();
        for variant in &variants {
            let _ = write!(reads, "\"{variant}\" => Self::{variant}, ");
            let _ = write!(writes, "Self::{variant} => \"{variant}\", ");
        }
        Some(format!(
            "\nimpl TacModel for crate::models::{name} {{\n    fn read(value: &TacValue) -> Self {{\n        match value.as_text().as_str() {{ {reads}_ => Self::{first} }}\n    }}\n    fn write(self) -> TacValue {{\n        TacValue::Text(String::from(match self {{ {writes}}}))\n    }}\n}}\n"
        ))
    }
}
