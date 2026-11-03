use super::model_files::{ModelFolder, ModelParser};
use super::{Model, ModelField, ModelTable, Shape};
use crate::ascii_name::AsciiName;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct Converters;

impl Converters {
    pub(crate) fn program_text(extension: &str, root: &Path) -> Result<String, String> {
        let files = ModelFolder::read(root, extension)?;
        Ok(Self::program_text_of(extension, &files))
    }

    fn program_text_of(extension: &str, files: &[(String, String)]) -> String {
        match extension {
            "dart" => DartConverters::text(files),
            "rb" => RubyConverters::text(files),
            "lua" => crate::lua_source::LuaModels::converters(&Self::records(files, extension)),
            _ => String::new(),
        }
    }

    pub(crate) fn rust_model_text(source: &str) -> String {
        RustConverters::text(source)
    }

    fn records(files: &[(String, String)], extension: &str) -> Vec<(String, super::Record)> {
        Self::models(files, extension)
            .into_iter()
            .filter_map(Self::record_entry)
            .collect()
    }

    fn models(files: &[(String, String)], extension: &str) -> Vec<(String, Model)> {
        files
            .iter()
            .flat_map(|(_, source)| ModelParser::models(extension, source))
            .collect()
    }

    fn record_entry((name, model): (String, Model)) -> Option<(String, super::Record)> {
        match model {
            Model::Record(record) => Some((name, record)),
            Model::Alias(_) => None,
        }
    }

    pub(crate) fn program_imports(extension: &str, root: &Path) -> Result<String, String> {
        let files = (extension == "dart")
            .then(|| ModelFolder::read(root, extension))
            .transpose()?;
        Ok(files.map_or_else(String::new, |files| DartConverters::imports(&files)))
    }

    pub(crate) fn tac_text(extension: &str, root: &Path) -> Result<String, String> {
        let files = ModelFolder::read_realm(root, "client", extension)?;
        Ok(Self::tac_text_of(extension, files))
    }

    fn tac_text_of(extension: &str, files: Vec<(String, String)>) -> String {
        match extension {
            "rs" => super::RustTacModels::of(&files).text(),
            "cs" => TacCSharpContext::text(&files),
            "swift" => SwiftCodable::text(&files),
            "dart" => DartConverters::tac_text(files),
            "lua" => crate::lua_source::LuaModels::converters(&Self::records(&files, extension)),
            _ => String::new(),
        }
    }

    pub(crate) fn require_rust_program(
        language: crate::project::NativeCompanion,
        root: &Path,
        unit: (&Path, &str),
        events: Option<&crate::events::EventModule>,
    ) -> Result<(), crate::Failure> {
        Self::rust_program(language, root, unit, events)
    }

    fn rust_program(
        language: crate::project::NativeCompanion,
        root: &Path,
        unit: (&Path, &str),
        events: Option<&crate::events::EventModule>,
    ) -> Result<(), crate::Failure> {
        if language != crate::project::NativeCompanion::Rust {
            return Ok(());
        }
        Self::require_rust_events(root, unit.0, events)?;
        Self::require_rust_mappings(root, unit.0, unit.1)
    }

    pub(crate) fn require_rust_selected_owner(
        language: crate::project::NativeCompanion,
        root: &Path,
        unit: (&Path, &str),
        excluded: &[String],
    ) -> Result<(), crate::Failure> {
        Self::rust_selected_owner(language, root, unit, excluded)
    }

    fn rust_selected_owner(
        language: crate::project::NativeCompanion,
        root: &Path,
        unit: (&Path, &str),
        excluded: &[String],
    ) -> Result<(), crate::Failure> {
        if language != crate::project::NativeCompanion::Rust {
            return Ok(());
        }
        Self::require_rust_owner(root, unit.1, unit.0, excluded)
    }

    pub(crate) fn require_rust_owner(
        root: &Path,
        source: &str,
        path: &Path,
        excluded: &[String],
    ) -> Result<(), crate::Failure> {
        let kinds = crate::companion::RustModelType::owner_types(source, path, excluded)?;
        Self::require_rust_types(root, &kinds, path)
    }

    pub(crate) fn require_rust_types(
        root: &Path,
        kinds: &[String],
        path: &Path,
    ) -> Result<(), crate::Failure> {
        let files = ModelFolder::read_realm(root, "client", "rs").map_err(|message| {
            crate::Failure::one(crate::failure::diagnostic(
                1405,
                message,
                None,
                crate::failure::source_span(&path.to_string_lossy(), 0, 1),
            ))
        })?;
        super::RustTacModels::of(&files).require(kinds, path)
    }

    pub(crate) fn require_rust_mappings(
        root: &Path,
        path: &Path,
        source: &str,
    ) -> Result<(), crate::Failure> {
        let kinds = crate::behavior::PlannedMethod::rust_mapping_types(path, source)?;
        Self::require_rust_types(root, &kinds, path)
    }

    pub(crate) fn require_rust_events(
        root: &Path,
        path: &Path,
        module: Option<&crate::events::EventModule>,
    ) -> Result<(), crate::Failure> {
        let kinds = module
            .into_iter()
            .flat_map(|module| module.types.iter())
            .flat_map(|kind| {
                kind.listeners
                    .iter()
                    .flat_map(|listener| listener.events.iter().chain(listener.returns.iter()))
                    .chain(kind.streams.iter().map(|stream| &stream.event))
            })
            .cloned()
            .collect::<Vec<_>>();
        Self::require_rust_types(root, &kinds, path)
    }

    pub(crate) fn csharp_tac_test_text(root: &Path) -> Result<String, String> {
        let files = ModelFolder::read_realm(root, "client", "cs")?;
        Ok(TacCSharpContext::test_text(&files))
    }

    pub(crate) fn dart_tac_test_text(root: &Path) -> Result<String, String> {
        let files = ModelFolder::read_realm(root, "client", "dart")?;
        Ok(DartConverters::text(&files))
    }
}

struct TacCSharpContext;

impl TacCSharpContext {
    fn text(files: &[(String, String)]) -> String {
        let mut output = Self::model_lines(files);
        Self::web_events(&mut output);
        Self::finish(output)
    }

    fn test_text(files: &[(String, String)]) -> String {
        Self::finish(Self::model_lines(files))
    }

    fn model_lines(files: &[(String, String)]) -> String {
        let mut output = String::from(
            "[System.Text.Json.Serialization.JsonSourceGenerationOptions(PropertyNamingPolicy = System.Text.Json.Serialization.JsonKnownNamingPolicy.CamelCase, UseStringEnumConverter = true)]\n[System.Text.Json.Serialization.JsonSerializable(typeof(string))]\n",
        );
        for (_, source) in files {
            Self::model_attributes(&mut output, source);
        }
        output
    }

    fn model_attributes(output: &mut String, source: &str) {
        for (name, _) in ModelParser::models("cs", source) {
            let _ = writeln!(
                output,
                "[System.Text.Json.Serialization.JsonSerializable(typeof(Models.{name}))]"
            );
        }
    }

    fn finish(mut output: String) -> String {
        output.push_str("internal partial class TacModelContext : System.Text.Json.Serialization.JsonSerializerContext {}\n");
        output
    }

    fn web_events(output: &mut String) {
        for name in super::WebEventModels::names() {
            let _ = writeln!(
                output,
                "[System.Text.Json.Serialization.JsonSerializable(typeof({name}))]"
            );
        }
    }
}

struct SwiftCodable;

impl SwiftCodable {
    const MODIFIERS: [&'static str; 5] =
        ["public ", "internal ", "fileprivate ", "private ", "final "];

    fn text(files: &[(String, String)]) -> String {
        let mut output = String::new();
        let missing = files
            .iter()
            .flat_map(|(_, source)| source.lines().filter_map(Self::declaration))
            .filter(|(_, header)| !Self::conforms(header));
        for (name, _) in missing {
            let _ = writeln!(output, "extension {name}: Codable {{}}");
        }
        output
    }

    fn conforms(header: &str) -> bool {
        header.contains("Codable") || header.contains("Decodable")
    }

    fn declaration(line: &str) -> Option<(&str, &str)> {
        let rest = Self::MODIFIERS.iter().fold(line.trim(), |rest, modifier| {
            rest.strip_prefix(modifier).unwrap_or(rest)
        });
        let header = rest
            .strip_prefix("struct ")
            .or_else(|| rest.strip_prefix("enum "))?;
        let end = header
            .find(|character: char| !Self::is_name_character(character))
            .unwrap_or(header.len());
        let name = &header[..end];
        (!name.is_empty()).then(|| (name, &header[end..]))
    }

    fn is_name_character(character: char) -> bool {
        [character.is_alphanumeric(), character == '_'].contains(&true)
    }
}

pub(super) struct Declarations;

impl Declarations {
    pub(super) fn variants(source: &str, name: &str) -> Option<Vec<String>> {
        Self::enum_variants(source, name)
    }

    fn enum_variants(source: &str, name: &str) -> Option<Vec<String>> {
        let header = format!("enum {name}");
        let at = source.find(&header)?;
        let open = at + source[at..].find('{')?;
        let close = open + source[open..].find('}')?;
        source[open + 1..close]
            .split([',', ';'])
            .map(|part| part.split('=').next().unwrap_or("").trim())
            .filter(|variant| !variant.is_empty())
            .map(Self::variant)
            .collect()
    }

    fn variant(text: &str) -> Option<String> {
        text.bytes()
            .all(Self::is_word_byte)
            .then(|| text.to_owned())
    }

    fn is_word_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }
}

struct RustConverters;

impl RustConverters {
    fn text(source: &str) -> String {
        ModelParser::models("rs", source)
            .into_iter()
            .map(|(name, model)| Self::model_impls(source, &name, &model))
            .collect()
    }

    fn model_impls(source: &str, name: &str, model: &Model) -> String {
        match model {
            Model::Record(record) => Self::record_impls(name, &record.fields),
            Model::Alias(_) => Self::enumeration_impls(source, name),
        }
    }

    fn record_impls(name: &str, fields: &[ModelField]) -> String {
        let mut reads = Vec::new();
        let mut writes = Vec::new();
        for field in fields {
            let field_name = &field.name;
            reads.push(format!(
                "{field_name}: YonRead::read(value.get(\"{field_name}\"))"
            ));
            writes.push(format!(
                "(String::from(\"{field_name}\"), Json::from(value.{field_name}))"
            ));
        }
        format!(
            "\nimpl YonRead for {name} {{\n    fn read(value: &Json) -> Self {{\n        Self {{ {} }}\n    }}\n}}\nimpl From<{name}> for Json {{\n    fn from(value: {name}) -> Self {{\n        Json::Map(vec![{}])\n    }}\n}}\n",
            reads.join(", "),
            writes.join(", ")
        )
    }

    fn enumeration_impls(source: &str, name: &str) -> String {
        let variants = Declarations::enum_variants(source, name).unwrap_or_default();
        variants.first().map_or_else(String::new, |first| {
            Self::variant_impls(name, first, &variants)
        })
    }

    fn variant_impls(name: &str, first: &str, variants: &[String]) -> String {
        let mut reads = String::new();
        let mut writes = String::new();
        for variant in variants {
            let _ = write!(reads, "\"{variant}\" => Self::{variant}, ");
            let _ = write!(writes, "{name}::{variant} => Json::from(\"{variant}\"), ");
        }
        format!(
            "\nimpl YonRead for {name} {{\n    fn read(value: &Json) -> Self {{\n        match value.text().as_str() {{ {reads}_ => Self::{first} }}\n    }}\n}}\nimpl From<{name}> for Json {{\n    fn from(value: {name}) -> Self {{\n        match value {{ {writes}}}\n    }}\n}}\n"
        )
    }
}

struct RubyConverters;

impl RubyConverters {
    fn text(files: &[(String, String)]) -> String {
        let entries: Vec<String> = files
            .iter()
            .flat_map(|(_, source)| ModelParser::models("rb", source))
            .filter_map(|(name, model)| Self::entry(&name, model))
            .collect();
        format!(
            "module YonGenerated\n  def self.field_types\n    {{\n{}\n    }}\n  end\nend\n",
            entries.join(",\n")
        )
    }

    fn entry(name: &str, model: Model) -> Option<String> {
        let Model::Record(record) = model else {
            return None;
        };
        let fields: Vec<String> = record.fields.iter().filter_map(Self::field_entry).collect();
        (!fields.is_empty()).then(|| format!("      {name} => {{ {} }}", fields.join(", ")))
    }

    fn field_entry(field: &ModelField) -> Option<String> {
        let descriptor = Self::descriptor(&field.shape)?;
        Some(format!("\"{}\" => {descriptor}", field.name))
    }

    fn descriptor(shape: &Shape) -> Option<String> {
        match shape {
            Shape::Model(name) => Some(name.clone()),
            Shape::List(item) => Self::descriptor(item).map(|item| format!("[{item}]")),
            Shape::Optional(inner) => Self::descriptor(inner),
            _ => None,
        }
    }
}

struct DartConverters;

impl DartConverters {
    const SHARED_LIBRARY: &str = "tachyon.dart";

    fn imports(files: &[(String, String)]) -> String {
        let mut imports = String::new();
        for (path, _) in files {
            let relative = path.strip_prefix("server/").unwrap_or(path);
            let _ = writeln!(imports, "import '{relative}';");
        }
        imports
    }

    fn text(files: &[(String, String)]) -> String {
        let table = ModelTable::from_files(files);
        let models = Self::models(files);
        let parents: BTreeMap<String, Vec<String>> = models
            .iter()
            .filter_map(|(_, name, model)| Self::parent_entry(name, model))
            .collect();
        let reads: String = models
            .iter()
            .map(|(source, name, model)| Self::model_read(&table, source, name, model))
            .collect();
        let aliases: String = models
            .iter()
            .map(|(_, name, _)| Self::alias(name))
            .collect();
        let mut records: Vec<(&str, &[ModelField])> = models
            .iter()
            .filter_map(|(_, name, model)| Self::joined_record(&table, name, model))
            .collect();
        records.sort_by_key(|(name, _)| std::cmp::Reverse(Self::ancestors(&parents, name)));
        let writes: String = records
            .iter()
            .map(|(name, fields)| Self::record_write(name, fields))
            .collect();
        format!(
            "{reads}\n{aliases}\nObject? tachyonPlainModel(Object? value) => __yon_json(value);\nObject? __yon_json(Object? value) {{\n{writes}  if (value is Enum) return value.name;\n  if (value is List) return value.map(__yon_json).toList();\n  if (value is Map) return value.map((key, item) => MapEntry(key, __yon_json(item)));\n  return value;\n}}\n"
        )
    }

    fn models(files: &[(String, String)]) -> Vec<(&str, String, Model)> {
        files
            .iter()
            .flat_map(|(_, source)| Self::source_models(source))
            .collect()
    }

    fn source_models(source: &str) -> impl Iterator<Item = (&str, String, Model)> {
        ModelParser::models("dart", source)
            .into_iter()
            .map(move |(name, model)| (source, name, model))
    }

    fn record_of(model: &Model) -> Option<&super::Record> {
        match model {
            Model::Record(record) => Some(record),
            Model::Alias(_) => None,
        }
    }

    fn parent_entry(name: &str, model: &Model) -> Option<(String, Vec<String>)> {
        let record = Self::record_of(model)?;
        Some((name.to_owned(), record.bases.clone()))
    }

    fn alias(name: &str) -> String {
        format!("{name} tachyonReadModel{name}(Object? value) => __yon_read_{name}(value);\n")
    }

    fn joined_fields<'a>(table: &'a ModelTable, name: &str) -> Option<&'a [ModelField]> {
        let joined = table.models.get(name).and_then(Self::record_of)?;
        Some(&joined.fields)
    }

    fn joined_record<'a>(
        table: &'a ModelTable,
        name: &'a str,
        model: &Model,
    ) -> Option<(&'a str, &'a [ModelField])> {
        Self::record_of(model)?;
        Some((name, Self::joined_fields(table, name)?))
    }

    fn model_read(table: &ModelTable, source: &str, name: &str, model: &Model) -> String {
        match model {
            Model::Record(_) => Self::joined_fields(table, name)
                .map_or_else(String::new, |fields| {
                    Self::record_read(source, name, fields)
                }),
            Model::Alias(_) => Self::enumeration_read(source, name),
        }
    }

    fn ancestors(parents: &BTreeMap<String, Vec<String>>, name: &str) -> usize {
        let lineage = std::iter::successors(Some(name), |current| Self::parent(parents, current));
        lineage.skip(1).take(ModelTable::MAX_DEPTH + 1).count()
    }

    fn parent<'a>(parents: &'a BTreeMap<String, Vec<String>>, name: &str) -> Option<&'a str> {
        let parent = parents.get(name).and_then(|bases| bases.first())?;
        Some(parent.as_str())
    }

    fn record_read(source: &str, name: &str, fields: &[ModelField]) -> String {
        let constructed = Self::declares_constructor(source, name)
            .then(|| Self::constructed_record(source, name, fields));
        constructed.unwrap_or_else(|| Self::assigned_record(source, name, fields))
    }

    fn constructed_record(source: &str, name: &str, fields: &[ModelField]) -> String {
        let (named, order) = Self::constructor(source, name, fields);
        let arguments: Vec<String> = order
            .iter()
            .filter_map(|field_name| fields.iter().find(|field| &field.name == field_name))
            .map(|field| Self::argument(named, field))
            .collect();
        format!(
            "\n{name} __yon_read_{name}(Object? value) {{\n  final map = value is Map ? value : const <String, Object?>{{}};\n  return {name}({});\n}}\n",
            arguments.join(", ")
        )
    }

    fn argument(named: bool, field: &ModelField) -> String {
        let value = Self::read(&field.shape, &format!("map['{}']", field.name));
        let labelled = named.then(|| format!("{}: {value}", field.name));
        labelled.unwrap_or(value)
    }

    fn assigned_record(source: &str, name: &str, fields: &[ModelField]) -> String {
        let mut assignments = String::new();
        for field in fields
            .iter()
            .filter(|field| !Self::is_final(source, &field.name))
        {
            let value = Self::read(&field.shape, &format!("map['{}']", field.name));
            let _ = write!(assignments, "..{} = {value}", field.name);
        }
        format!(
            "\n{name} __yon_read_{name}(Object? value) {{\n  final map = value is Map ? value : const <String, Object?>{{}};\n  return {name}(){assignments};\n}}\n"
        )
    }

    fn declares_constructor(source: &str, name: &str) -> bool {
        [format!("{name}("), format!("{name}.")]
            .iter()
            .flat_map(|marker| source.match_indices(marker.as_str()))
            .any(|(at, _)| Self::starts_a_word(source, at))
    }

    fn starts_a_word(source: &str, at: usize) -> bool {
        !source[..at]
            .chars()
            .next_back()
            .is_some_and(Self::is_word_character)
    }

    fn is_word_character(character: char) -> bool {
        [character.is_ascii_alphanumeric(), character == '_'].contains(&true)
    }

    fn is_final(source: &str, field: &str) -> bool {
        source.lines().any(|line| Self::declares_final(line, field))
    }

    fn declares_final(line: &str, field: &str) -> bool {
        let mut words = line
            .split(|character: char| !Self::is_word_character(character))
            .filter(|word| !word.is_empty());
        words.next() == Some("final") && words.any(|word| word == field)
    }

    fn tac_text(mut files: Vec<(String, String)>) -> String {
        files.push((
            String::from(Self::SHARED_LIBRARY),
            super::WebEventSources::dart(),
        ));
        format!("{}{}", Self::text(&files), Self::event_names(&files))
    }

    fn event_names(files: &[(String, String)]) -> String {
        let mut checks = String::new();
        let records = files
            .iter()
            .flat_map(|(_, source)| ModelParser::models("dart", source))
            .filter(|(_, model)| Self::record_of(model).is_some());
        for (name, _) in records {
            let _ = writeln!(checks, "  if (value is {name}) return '{name}';");
        }
        format!("\nString? __tac_event_name(Object? value) {{\n{checks}  return null;\n}}\n")
    }

    fn record_write(name: &str, fields: &[ModelField]) -> String {
        let entries: Vec<String> = fields
            .iter()
            .map(|field| format!("'{0}': __yon_json(value.{0})", field.name))
            .collect();
        format!(
            "  if (value is {name}) return {{{}}};\n",
            entries.join(", ")
        )
    }

    fn enumeration_read(source: &str, name: &str) -> String {
        Declarations::enum_variants(source, name).map_or_else(String::new, |_| {
            format!(
                "\n{name} __yon_read_{name}(Object? value) => {name}.values.byName('$value');\n"
            )
        })
    }

    fn constructor(source: &str, name: &str, fields: &[ModelField]) -> (bool, Vec<String>) {
        let marker = format!("{name}(");
        let parameters = Self::constructor_parameters(source, &marker);
        parameters.map_or_else(
            || {
                (
                    true,
                    fields.iter().map(|field| field.name.clone()).collect(),
                )
            },
            Self::parameter_order,
        )
    }

    fn constructor_parameters<'a>(source: &'a str, marker: &str) -> Option<&'a str> {
        source
            .match_indices(marker)
            .filter(|&(at, _)| Self::starts_a_word(source, at))
            .map(|(at, _)| &source[at + marker.len()..])
            .map_while(|after| after.find(')').map(|close| &after[..close]))
            .find(|list| Self::assigns_fields(list))
    }

    fn assigns_fields(list: &str) -> bool {
        list.contains("this.") || list.contains("super.")
    }

    fn parameter_order(list: &str) -> (bool, Vec<String>) {
        let named = list.trim_start().starts_with('{');
        let order = list.split(',').filter_map(Self::assigned_field).collect();
        (named, order)
    }

    fn assigned_field(part: &str) -> Option<String> {
        let field = part
            .split("this.")
            .nth(1)
            .or_else(|| part.split("super.").nth(1))?;
        Some(AsciiName::leading(field).to_owned())
    }

    fn read(shape: &Shape, value: &str) -> String {
        match shape {
            Shape::Any => value.to_owned(),
            Shape::Text => format!("({value} as String)"),
            Shape::Integer => format!("({value} as num).toInt()"),
            Shape::Number => format!("({value} as num).toDouble()"),
            Shape::Boolean => format!("({value} as bool)"),
            Shape::Optional(inner) => {
                format!("({value} == null ? null : {})", Self::read(inner, value))
            }
            Shape::List(item) => format!(
                "({value} as List).map((item) => {}).toList()",
                Self::read(item, "item")
            ),
            Shape::Model(name) => format!("__yon_read_{name}({value})"),
        }
    }
}

pub(crate) struct DartValues;

impl DartValues {
    pub(crate) fn read(shape: &Shape, value: &str) -> String {
        DartConverters::read(shape, value)
    }
}
