use super::converters::Declarations;
use super::model_files::ModelParser;
use super::rust_model_traversal::RustModelTraversal;
use super::{Model, ModelTable, RustField, WebEventModels};
use crate::Failure;
use crate::failure::{diagnostic, source_span};
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
    const EVENT_PREFIXES: [&'static str; 2] = ["crate::", "::crate::"];
    const ALIAS_KEYWORD: &'static str = "type";
    const ENUM_PREFIX: &'static str = "enum ";

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
            self.check_type(kind, &mut RustModelTraversal::default())
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
        self.check_type(name, &mut RustModelTraversal::default())
            .ok()?;
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

    fn check_type(&self, kind: &str, seen: &mut RustModelTraversal) -> Result<(), Failure> {
        let compact = Self::compact_type(kind)?;
        self.check_compact(&compact, seen)
    }

    fn check_compact(&self, kind: &str, seen: &mut RustModelTraversal) -> Result<(), Failure> {
        let captured = self.check_captured(kind, seen).transpose()?;
        captured.map_or_else(|| self.check_intrinsic(kind, seen), |()| Ok(()))
    }

    fn check_captured(
        &self,
        kind: &str,
        seen: &mut RustModelTraversal,
    ) -> Option<Result<(), Failure>> {
        let name = Self::captured_name(kind)?;
        self.fields(name)
            .map(|_| self.check_record(name, seen))
            .or_else(|| {
                self.alias_target(name)
                    .map(|target| self.check_alias(name, &target, seen))
            })
            .or_else(|| {
                self.captured_enum(name)
                    .map(|(path, variants, range)| Self::check_enum(name, path, &variants, range))
            })
    }

    fn captured_name(kind: &str) -> Option<&str> {
        Self::MODEL_PREFIXES
            .iter()
            .find_map(|prefix| kind.strip_prefix(prefix))
            .or_else(|| (!kind.contains("::")).then_some(kind))
    }

    fn check_intrinsic(&self, kind: &str, seen: &mut RustModelTraversal) -> Result<(), Failure> {
        if Self::scalar(kind) {
            return Ok(());
        }
        self.check_container(kind, seen)
    }

    fn scalar(kind: &str) -> bool {
        Self::SCALARS
            .iter()
            .copied()
            .find(|supported| *supported == kind)
            .or_else(|| {
                Self::PRIMITIVE_PREFIXES
                    .iter()
                    .find_map(|prefix| kind.strip_prefix(prefix))
                    .filter(|primitive| Self::PRIMITIVES.contains(primitive))
            })
            .is_some()
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

    fn check_container(&self, kind: &str, seen: &mut RustModelTraversal) -> Result<(), Failure> {
        let checked = Self::item(kind)
            .map(|item| self.check_type(item, seen))
            .transpose()?;
        checked.map_or_else(|| Self::check_supplied(kind), |()| Ok(()))
    }

    fn item(kind: &str) -> Option<&str> {
        let (base, rest) = kind.split_once('<')?;
        Self::CONTAINERS.contains(&base).then_some(())?;
        rest.strip_suffix('>')
    }

    fn check_supplied(kind: &str) -> Result<(), Failure> {
        if Self::supplied(kind) {
            return Ok(());
        }
        Err(Self::unsupported(kind))
    }

    fn supplied(kind: &str) -> bool {
        let name = Self::EVENT_PREFIXES
            .iter()
            .find_map(|prefix| kind.strip_prefix(prefix))
            .unwrap_or(kind);
        WebEventModels::names().contains(&name.to_owned())
    }

    fn alias_target(&self, name: &str) -> Option<String> {
        self.files
            .iter()
            .find_map(|(_, source)| Self::alias(source, name))
    }

    fn captured_enum(&self, name: &str) -> Option<(&str, Vec<String>, std::ops::Range<usize>)> {
        self.files.iter().find_map(|(path, source)| {
            Self::enum_info(source, name).map(|(variants, range)| (path.as_str(), variants, range))
        })
    }

    fn check_enum(
        name: &str,
        path: &str,
        variants: &[String],
        range: std::ops::Range<usize>,
    ) -> Result<(), Failure> {
        if variants.is_empty() {
            return Err(Self::failure(
                Path::new(path),
                range,
                &format!(
                    "Rust Tac empty enum '{name}' has no supported wire codec; keep it inside Rust."
                ),
            ));
        }
        Ok(())
    }

    fn check_alias(
        &self,
        name: &str,
        target: &str,
        seen: &mut RustModelTraversal,
    ) -> Result<(), Failure> {
        Self::limit(seen.knows_alias(name), seen)?;
        Self::enter_alias(name, seen)?;
        let result = self.check_type(target, seen);
        seen.leave_alias(name);
        result
    }

    fn enter_alias(name: &str, seen: &mut RustModelTraversal) -> Result<(), Failure> {
        if !seen.enter_alias(name) {
            return Err(Self::unsupported(name));
        }
        Ok(())
    }

    fn alias(source: &str, name: &str) -> Option<String> {
        let code =
            crate::annotation_layout::masked_source(Path::new(Self::MASK_PATH), source).ok()?;
        code.match_indices(Self::ALIAS_KEYWORD)
            .find_map(|(at, _)| Self::alias_statement(&code, at, name))
    }

    fn alias_statement(code: &str, at: usize, name: &str) -> Option<String> {
        let after = Self::after_alias_keyword(code, at)?;
        let (statement, _) = after.split_once(';')?;
        let (declared, target) = statement.split_once('=')?;
        (declared.trim() == name).then(|| target.trim().to_owned())
    }

    fn after_alias_keyword(code: &str, at: usize) -> Option<&str> {
        let before = code[..at].chars().next_back();
        let after = &code[at + Self::ALIAS_KEYWORD.len()..];
        let starts_word = !before.is_some_and(Self::name_character);
        let ends_word = after.chars().next().is_some_and(char::is_whitespace);
        ([starts_word, ends_word] == [true, true]).then_some(after.trim_start())
    }

    fn name_character(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn check_record(&self, name: &str, seen: &mut RustModelTraversal) -> Result<(), Failure> {
        let (path, fields) = self.fields(name).ok_or_else(|| Self::unsupported(name))?;
        Self::limit(seen.knows_record(name), seen)?;
        seen.enter_record(name)
            .then_some(())
            .map(|()| self.record_fields(path, &fields, seen))
            .transpose()
            .map(|_| ())
    }

    fn record_fields(
        &self,
        path: &str,
        fields: &[RustField],
        seen: &mut RustModelTraversal,
    ) -> Result<(), Failure> {
        let mut captured = seen.for_fields();
        let result = self.check_fields(path, fields, &mut captured);
        seen.finish_fields(captured);
        result
    }

    fn limit(known: bool, seen: &RustModelTraversal) -> Result<(), Failure> {
        if [known, seen.count() < Self::MAX_MODELS].contains(&true) {
            return Ok(());
        }
        Err(Self::unsupported("model closure exceeds 256 declarations"))
    }

    fn check_fields(
        &self,
        path: &str,
        fields: &[RustField],
        seen: &mut RustModelTraversal,
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
            .map(|field| format!("{}: tac_read_field(value, {:?})", field.name, field.key()))
            .collect::<Vec<_>>();
        let writes = fields
            .iter()
            .map(|field| {
                format!(
                    "(String::from({:?}), TacModel::write(self.{}))",
                    field.key(),
                    field.name
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
        Self::enum_info(source, name).map(|(variants, _)| variants)
    }

    fn enum_info(source: &str, name: &str) -> Option<(Vec<String>, std::ops::Range<usize>)> {
        let code =
            crate::annotation_layout::masked_source(Path::new(Self::MASK_PATH), source).ok()?;
        let header = format!("{}{name}", Self::ENUM_PREFIX);
        let (at, _) = code
            .match_indices(&header)
            .find(|(at, _)| code[at + header.len()..].trim_start().starts_with('{'))?;
        let variants = Declarations::variants(&code[at..], name)?;
        let start = at + Self::ENUM_PREFIX.len();
        Some((variants, start..start + name.len()))
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
