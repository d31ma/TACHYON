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
            "rs" => Ok(RustConverters::text(&files)),
            "dart" => Ok(DartConverters::text(&files)),
            "rb" => Ok(RubyConverters::text(&files)),
            "lua" => Ok(crate::lua_source::LuaModels::converters(&Self::records(&files, extension))),
            _ => Ok(String::new()),
        }
    }

    fn records(files: &[(String, String)], extension: &str) -> Vec<(String, super::Record)> {
        files
            .iter()
            .flat_map(|(_, source)| ModelParser::models(extension, source))
            .filter_map(|(name, model)| match model {
                Model::Record(record) => Some((name, record)),
                Model::Alias(_) => None,
            })
            .collect()
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
            "rs" => Ok(TacRustConverters::text(&files)),
            "cs" => Ok(TacCSharpContext::text(&files)),
            "swift" => Ok(SwiftCodable::text(&files)),
            "dart" => Ok(DartConverters::tac_text(files)),
            _ => Ok(String::new()),
        }
    }
}

struct TacRustConverters;

impl TacRustConverters {
    fn text(files: &[(String, String)]) -> String {
        let mut output = String::new();
        for (_, source) in files {
            for (name, model) in ModelParser::models("rs", source) {
                match model {
                    Model::Record(record) => Self::record(&mut output, &name, &record.fields),
                    Model::Alias(_) => Self::enumeration(&mut output, source, &name),
                }
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
                "{field_name}: tac_read_field(value, \"{field_name}\")"
            ));
            writes.push(format!(
                "(String::from(\"{field_name}\"), TacModel::write(self.{field_name}))"
            ));
        }
        let _ = write!(
            output,
            "\nimpl TacModel for crate::models::{name} {{\n    fn read(value: &TacValue) -> Self {{\n        Self {{ {} }}\n    }}\n    fn write(self) -> TacValue {{\n        TacValue::Object(vec![{}])\n    }}\n}}\n",
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
            let _ = write!(writes, "Self::{variant} => \"{variant}\", ");
        }
        let _ = write!(
            output,
            "\nimpl TacModel for crate::models::{name} {{\n    fn read(value: &TacValue) -> Self {{\n        match value.as_text().as_str() {{ {reads}_ => Self::{first} }}\n    }}\n    fn write(self) -> TacValue {{\n        TacValue::Text(String::from(match self {{ {writes}}}))\n    }}\n}}\n"
        );
    }
}

struct TacCSharpContext;

impl TacCSharpContext {
    fn text(files: &[(String, String)]) -> String {
        let mut output = String::from(
            "[System.Text.Json.Serialization.JsonSourceGenerationOptions(PropertyNamingPolicy = System.Text.Json.Serialization.JsonKnownNamingPolicy.CamelCase, UseStringEnumConverter = true)]\n[System.Text.Json.Serialization.JsonSerializable(typeof(string))]\n",
        );
        for (_, source) in files {
            for (name, _) in ModelParser::models("cs", source) {
                let _ = writeln!(
                    output,
                    "[System.Text.Json.Serialization.JsonSerializable(typeof(Models.{name}))]"
                );
            }
        }
        Self::web_events(&mut output);
        output.push_str(
            "internal partial class TacModelContext : System.Text.Json.Serialization.JsonSerializerContext {}\n",
        );
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

struct Declarations;

impl Declarations {
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
    fn text(files: &[(String, String)]) -> String {
        let mut output = String::new();
        for (_, source) in files {
            for (name, model) in ModelParser::models("rs", source) {
                match model {
                    Model::Record(record) => Self::record(&mut output, &name, &record.fields),
                    Model::Alias(_) => Self::enumeration(&mut output, source, &name),
                }
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
            "\nimpl YonRead for crate::models::{name} {{\n    fn read(value: &Json) -> Self {{\n        Self {{ {} }}\n    }}\n}}\nimpl From<crate::models::{name}> for Json {{\n    fn from(value: crate::models::{name}) -> Self {{\n        Json::Map(vec![{}])\n    }}\n}}\n",
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
            let _ = write!(
                writes,
                "crate::models::{name}::{variant} => Json::from(\"{variant}\"), "
            );
        }
        let _ = write!(
            output,
            "\nimpl YonRead for crate::models::{name} {{\n    fn read(value: &Json) -> Self {{\n        match value.text().as_str() {{ {reads}_ => Self::{first} }}\n    }}\n}}\nimpl From<crate::models::{name}> for Json {{\n    fn from(value: crate::models::{name}) -> Self {{\n        match value {{ {writes}}}\n    }}\n}}\n"
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
        let mut records = Vec::new();
        for (_, source) in files {
            for (name, model) in ModelParser::models("dart", source) {
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
            "{reads}\nObject? __yon_json(Object? value) {{\n{writes}  if (value is Enum) return value.name;\n  if (value is List) return value.map(__yon_json).toList();\n  if (value is Map) return value.map((key, item) => MapEntry(key, __yon_json(item)));\n  return value;\n}}\n"
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
