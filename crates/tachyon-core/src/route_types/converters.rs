use super::model_files::{ModelFolder, ModelParser};
use super::{Model, ModelField, ModelTable, Shape};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;

pub(crate) struct Converters;

impl Converters {
    pub(crate) fn program_text(extension: &str, root: &Path) -> Result<String, String> {
        let files = ModelFolder::read(root, extension)?;
        match extension {
            "dart" => Ok(DartConverters::text(&files)),
            "rb" => Ok(RubyConverters::text(&files)),
            "lua" => Ok(crate::lua_source::LuaModels::converters(&Self::records(
                &files, extension,
            ))),
            _ => Ok(String::new()),
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
        if extension != "dart" {
            return Ok(String::new());
        }
        let files = ModelFolder::read(root, extension)?;
        Ok(DartConverters::imports(&files))
    }

    pub(crate) fn tac_text(extension: &str, root: &Path) -> Result<String, String> {
        let files = ModelFolder::read_realm(root, "client", extension)?;
        match extension {
            "rs" => Ok(super::RustTacModels::of(&files).text()),
            "cs" => Ok(TacCSharpContext::text(&files)),
            "swift" => Ok(SwiftCodable::text(&files)),
            "dart" => Ok(DartConverters::tac_text(files)),
            "lua" => Ok(crate::lua_source::LuaModels::converters(&Self::records(
                &files, extension,
            ))),
            _ => Ok(String::new()),
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
    fn text(files: &[(String, String)]) -> String {
        let mut output = String::new();
        for (_, source) in files {
            for line in source.lines() {
                let Some((name, header)) = Self::declaration(line) else {
                    continue;
                };
                let conforms = header.contains("Codable") || header.contains("Decodable");
                if !conforms {
                    let _ = writeln!(output, "extension {name}: Codable {{}}");
                }
            }
        }
        output
    }

    fn declaration(line: &str) -> Option<(&str, &str)> {
        let mut rest = line.trim();
        for modifier in ["public ", "internal ", "fileprivate ", "private ", "final "] {
            rest = rest.strip_prefix(modifier).unwrap_or(rest);
        }
        let header = rest
            .strip_prefix("struct ")
            .or_else(|| rest.strip_prefix("enum "))?;
        let end = header
            .find(|character: char| !(character.is_alphanumeric() || character == '_'))
            .unwrap_or(header.len());
        let name = &header[..end];
        if name.is_empty() {
            return None;
        }
        Some((name, &header[end..]))
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
        let mut variants = Vec::new();
        for part in source[open + 1..close].split([',', ';']) {
            let variant = part.split('=').next().unwrap_or("").trim();
            if variant.is_empty() {
                continue;
            }
            if !variant
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            {
                return None;
            }
            variants.push(variant.to_owned());
        }
        Some(variants)
    }
}

struct RustConverters;

impl RustConverters {
    fn text(source: &str) -> String {
        let mut output = String::new();
        for (name, model) in ModelParser::models("rs", source) {
            match model {
                Model::Record(record) => Self::record(&mut output, &name, &record.fields),
                Model::Alias(_) => Self::enumeration(&mut output, source, &name),
            }
        }
        output
    }

    fn record(output: &mut String, name: &str, fields: &[ModelField]) {
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
        let _ = write!(
            output,
            "\nimpl YonRead for {name} {{\n    fn read(value: &Json) -> Self {{\n        Self {{ {} }}\n    }}\n}}\nimpl From<{name}> for Json {{\n    fn from(value: {name}) -> Self {{\n        Json::Map(vec![{}])\n    }}\n}}\n",
            reads.join(", "),
            writes.join(", ")
        );
    }

    fn enumeration(output: &mut String, source: &str, name: &str) {
        let Some(variants) = Declarations::enum_variants(source, name) else {
            return;
        };
        let Some(first) = variants.first() else {
            return;
        };
        let mut reads = String::new();
        let mut writes = String::new();
        for variant in &variants {
            let _ = write!(reads, "\"{variant}\" => Self::{variant}, ");
            let _ = write!(writes, "{name}::{variant} => Json::from(\"{variant}\"), ");
        }
        let _ = write!(
            output,
            "\nimpl YonRead for {name} {{\n    fn read(value: &Json) -> Self {{\n        match value.text().as_str() {{ {reads}_ => Self::{first} }}\n    }}\n}}\nimpl From<{name}> for Json {{\n    fn from(value: {name}) -> Self {{\n        match value {{ {writes}}}\n    }}\n}}\n"
        );
    }
}

struct RubyConverters;

impl RubyConverters {
    fn text(files: &[(String, String)]) -> String {
        let mut entries = Vec::new();
        for (_, source) in files {
            for (name, model) in ModelParser::models("rb", source) {
                let Model::Record(record) = model else {
                    continue;
                };
                let mut fields = Vec::new();
                for field in &record.fields {
                    if let Some(descriptor) = Self::descriptor(&field.shape) {
                        fields.push(format!("\"{}\" => {descriptor}", field.name));
                    }
                }
                if !fields.is_empty() {
                    entries.push(format!("      {name} => {{ {} }}", fields.join(", ")));
                }
            }
        }
        format!(
            "module YonGenerated\n  def self.field_types\n    {{\n{}\n    }}\n  end\nend\n",
            entries.join(",\n")
        )
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
        let mut parents = BTreeMap::new();
        let mut reads = String::new();
        let mut aliases = String::new();
        let mut records = Vec::new();
        for (_, source) in files {
            for (name, model) in ModelParser::models("dart", source) {
                let _ = writeln!(
                    aliases,
                    "{name} tachyonReadModel{name}(Object? value) => __yon_read_{name}(value);"
                );
                match model {
                    Model::Record(record) => {
                        parents.insert(name.clone(), record.bases);
                        let Some(Model::Record(joined)) = table.models.get(&name) else {
                            continue;
                        };
                        Self::record(&mut reads, source, &name, &joined.fields);
                        records.push((name, joined.fields.clone()));
                    }
                    Model::Alias(_) => Self::enumeration(&mut reads, source, &name),
                }
            }
        }
        records.sort_by_key(|(name, _)| std::cmp::Reverse(Self::ancestors(&parents, name)));
        let mut writes = String::new();
        for (name, fields) in &records {
            Self::record_write(&mut writes, name, fields);
        }
        format!(
            "{reads}\n{aliases}\nObject? tachyonPlainModel(Object? value) => __yon_json(value);\nObject? __yon_json(Object? value) {{\n{writes}  if (value is Enum) return value.name;\n  if (value is List) return value.map(__yon_json).toList();\n  if (value is Map) return value.map((key, item) => MapEntry(key, __yon_json(item)));\n  return value;\n}}\n"
        )
    }

    fn ancestors(parents: &BTreeMap<String, Vec<String>>, name: &str) -> usize {
        let mut count = 0;
        let mut current = name;
        while let Some(parent) = parents.get(current).and_then(|bases| bases.first()) {
            count += 1;
            current = parent;
            if count > ModelTable::MAX_DEPTH {
                break;
            }
        }
        count
    }

    fn record(output: &mut String, source: &str, name: &str, fields: &[ModelField]) {
        let (named, order) = Self::constructor(source, name, fields);
        let mut arguments = Vec::new();
        for field_name in &order {
            let Some(field) = fields.iter().find(|field| &field.name == field_name) else {
                continue;
            };
            let value = Self::read(&field.shape, &format!("map['{}']", field.name));
            if named {
                arguments.push(format!("{}: {value}", field.name));
            } else {
                arguments.push(value);
            }
        }
        let _ = write!(
            output,
            "\n{name} __yon_read_{name}(Object? value) {{\n  final map = value is Map ? value : const <String, Object?>{{}};\n  return {name}({});\n}}\n",
            arguments.join(", ")
        );
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
        for (_, source) in files {
            for (name, model) in ModelParser::models("dart", source) {
                if matches!(model, Model::Record(_)) {
                    let _ = writeln!(checks, "  if (value is {name}) return '{name}';");
                }
            }
        }
        format!("\nString? __tac_event_name(Object? value) {{\n{checks}  return null;\n}}\n")
    }

    fn record_write(output: &mut String, name: &str, fields: &[ModelField]) {
        let mut entries = Vec::new();
        for field in fields {
            entries.push(format!("'{0}': __yon_json(value.{0})", field.name));
        }
        let _ = writeln!(
            output,
            "  if (value is {name}) return {{{}}};",
            entries.join(", ")
        );
    }

    fn enumeration(output: &mut String, source: &str, name: &str) {
        if Declarations::enum_variants(source, name).is_none() {
            return;
        }
        let _ = write!(
            output,
            "\n{name} __yon_read_{name}(Object? value) => {name}.values.byName('$value');\n"
        );
    }

    fn constructor(source: &str, name: &str, fields: &[ModelField]) -> (bool, Vec<String>) {
        let fallback: Vec<String> = fields.iter().map(|field| field.name.clone()).collect();
        let marker = format!("{name}(");
        let mut rest = source;
        while let Some(at) = rest.find(&marker) {
            let before = rest[..at].chars().next_back();
            let after = &rest[at + marker.len()..];
            rest = after;
            if before.is_some_and(|character| character.is_ascii_alphanumeric() || character == '_')
            {
                continue;
            }
            let Some(close) = after.find(')') else {
                break;
            };
            let list = &after[..close];
            if !list.contains("this.") && !list.contains("super.") {
                continue;
            }
            let named = list.trim_start().starts_with('{');
            let mut order = Vec::new();
            for part in list.split(',') {
                let field = part.split("this.").nth(1);
                let Some(field) = field.or_else(|| part.split("super.").nth(1)) else {
                    continue;
                };
                let field: String = field
                    .chars()
                    .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                    .collect();
                order.push(field);
            }
            return (named, order);
        }
        (true, fallback)
    }

    pub(crate) fn read(shape: &Shape, value: &str) -> String {
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
