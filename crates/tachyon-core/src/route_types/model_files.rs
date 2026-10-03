use super::{Model, ModelField, Record, Shape, TypeText};
use std::path::{Path, PathBuf};

mod alias_models;
mod ruby;
mod rust_field;
mod rust_fields;

pub(crate) use rust_field::RustField;

#[cfg(test)]
mod rust_fields_tests;

pub(super) struct ModelFolder;

impl ModelFolder {
    const MAX_FILES: usize = 256;
    const MAX_DEPTH: usize = 8;
    const MAX_TOTAL_BYTES: usize = 1024 * 1024;
    const ALSO_READS: [(&str, &str); 1] = [("js", "ts")];

    pub(super) fn read(root: &Path, extension: &str) -> Result<Vec<(String, String)>, String> {
        Self::read_realm(root, "server", extension)
    }

    pub(super) fn read_realm(
        root: &Path,
        realm: &str,
        extension: &str,
    ) -> Result<Vec<(String, String)>, String> {
        let mut files = Vec::new();
        let mut total = 0;
        let folder = root.join(realm).join("models");
        let portable = PathBuf::from(format!("{realm}/models"));
        Self::read_directory(&folder, &portable, extension, 0, &mut files, &mut total)?;
        files.sort();
        Ok(files)
    }

    fn read_directory(
        folder: &Path,
        portable: &Path,
        extension: &str,
        depth: usize,
        files: &mut Vec<(String, String)>,
        total: &mut usize,
    ) -> Result<(), String> {
        if depth > Self::MAX_DEPTH {
            return Err(format!(
                "{} is nested more than 8 folders deep",
                Self::top(portable)
            ));
        }
        let Ok(entries) = std::fs::read_dir(folder) else {
            return Ok(());
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        for path in paths {
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            let name = path.file_name().unwrap_or_default();
            let child = portable.join(name);
            if metadata.is_dir() {
                Self::read_directory(&path, &child, extension, depth + 1, files, total)?;
                continue;
            }
            let found = path.extension().and_then(|value| value.to_str());
            if !metadata.is_file() || !Self::reads(extension, found.unwrap_or_default()) {
                continue;
            }
            let source = std::fs::read_to_string(&path)
                .map_err(|_| format!("Cannot read {}", child.display()))?;
            *total += source.len();
            if files.len() >= Self::MAX_FILES || *total > Self::MAX_TOTAL_BYTES {
                return Err(format!(
                    "{} holds more than 256 files or 1 MiB",
                    Self::top(portable)
                ));
            }
            files.push((child.to_string_lossy().replace('\\', "/"), source));
        }
        Ok(())
    }

    fn reads(extension: &str, found: &str) -> bool {
        found == extension || Self::ALSO_READS.contains(&(extension, found))
    }

    pub(super) fn language(path: &str) -> &str {
        Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
    }

    fn top(portable: &Path) -> String {
        let Some(realm) = portable.components().next() else {
            return String::from("models");
        };
        format!("{}/models", realm.as_os_str().to_string_lossy())
    }
}

pub(super) struct ModelParser;

impl ModelParser {
    pub(super) fn models(extension: &str, source: &str) -> Vec<(String, Model)> {
        if extension == crate::fsharp_source::FsharpSource::EXTENSION {
            return Self::models("cs", crate::fsharp_source::FsharpShadow::of(source).text());
        }
        if extension == crate::lua_source::LuaSource::EXTENSION {
            return Self::models("rb", crate::lua_source::LuaShadow::joined(source).text());
        }
        let typed = if extension == "php" {
            super::PhpDoc::typed_arrays(source)
        } else {
            source.to_owned()
        };
        let source = typed.as_str();
        let code = Self::without_comments(extension, source);
        let extension = crate::groovy_source::GroovySource::syntax(extension);
        match extension {
            "ts" => TypeScriptModels::read(&code),
            "js" => Self::jsdoc_models(source),
            "rb" => ruby::RubyModels::read(source),
            "py" => PythonModels::read(&code),
            "go" => GoModels::read(&code),
            "rs" => [
                rust_fields::RustFields::models(&code, BracedModels::read(extension, &code)),
                alias_models::AliasModels::read(extension, &code),
            ]
            .concat(),
            "java" | "kt" | "cs" | "dart" | "php" | "swift" => [
                BracedModels::read(extension, &code),
                alias_models::AliasModels::read(extension, &code),
            ]
            .concat(),
            _ => Vec::new(),
        }
    }

    pub(super) fn rust_model_fields(source: &str) -> Vec<(String, Vec<RustField>)> {
        let code = Self::without_comments("rs", source);
        rust_fields::RustFields::read_models(&code)
    }

    pub(super) fn jsdoc_models(source: &str) -> Vec<(String, Model)> {
        let mut models = Vec::new();
        let mut rest = source;
        while let Some(start) = rest.find("/**") {
            let Some(length) = rest[start..].find("*/") else {
                break;
            };
            let block = &rest[start + 3..start + length];
            if let Some(model) = JsDocModels::typedef(block) {
                models.push(model);
            }
            rest = &rest[start + length + 2..];
        }
        models
    }

    fn without_comments(extension: &str, source: &str) -> String {
        let mask = crate::lexical::code_mask(extension, source);
        let bytes = source.as_bytes();
        let mut output = bytes.to_vec();
        let mut at = 0;
        while at < bytes.len() {
            if mask.code.get(at).copied().unwrap_or(true) {
                at += 1;
                continue;
            }
            let start = at;
            while at < bytes.len() && !mask.code.get(at).copied().unwrap_or(true) {
                at += 1;
            }
            let keeps_text = matches!(bytes[start], b'"' | b'\'' | b'`');
            if keeps_text {
                continue;
            }
            for byte in &mut output[start..at] {
                if !matches!(*byte, b'\n' | b'\r') {
                    *byte = b' ';
                }
            }
        }
        String::from_utf8(output).unwrap_or_default()
    }
}

impl Record {
    fn add(&mut self, extension: &str, name: &str, type_text: &str, required: bool) {
        match TypeText::shape(extension, type_text) {
            Ok(shape) => {
                let required = required && !shape.is_optional();
                self.fields.push(ModelField {
                    name: name.to_owned(),
                    shape,
                    required,
                });
            }
            Err(_) => self.open = true,
        }
    }

    fn extend(&mut self, other: Self) {
        self.fields.extend(other.fields);
        self.bases.extend(other.bases);
        self.open |= other.open;
        if self.unreadable.is_none() {
            self.unreadable = other.unreadable;
        }
    }
}

#[derive(Default)]
struct Quotes {
    open: Option<u8>,
    escaped: bool,
}

impl Quotes {
    fn inside(&mut self, extension: &str, bytes: &[u8], at: usize) -> bool {
        let byte = bytes[at];
        if let Some(open) = self.open {
            if self.escaped {
                self.escaped = false;
            } else if byte == b'\\' && open != b'`' {
                self.escaped = true;
            } else if byte == open {
                self.open = None;
            }
            return true;
        }
        let starts = match byte {
            b'"' | b'`' => true,
            b'\'' => extension != "rs" || Self::rust_character(bytes, at),
            _ => false,
        };
        if starts {
            self.open = Some(byte);
        }
        starts
    }

    fn rust_character(bytes: &[u8], at: usize) -> bool {
        bytes.get(at + 1) == Some(&b'\\') || bytes.get(at + 2) == Some(&b'\'')
    }
}

struct Members;

impl Members {
    fn depth_change(bytes: &[u8], at: usize) -> i32 {
        match bytes[at] {
            b'<' | b'[' | b'(' | b'{' => 1,
            b'>' if at > 0 && matches!(bytes[at - 1], b'=' | b'-') => 0,
            b'>' | b']' | b')' | b'}' => -1,
            _ => 0,
        }
    }

    fn split(extension: &str, body: &str) -> Vec<String> {
        let bytes = body.as_bytes();
        let mut quotes = Quotes::default();
        let mut parts = Vec::new();
        let mut depth = 0_i32;
        let mut start = 0;
        for at in 0..bytes.len() {
            if quotes.inside(extension, bytes, at) {
                continue;
            }
            depth += Self::depth_change(bytes, at);
            if depth == 0 && matches!(bytes[at], b';' | b',' | b'\n') {
                Self::push_part(&mut parts, &body[start..at]);
                start = at + 1;
            }
        }
        Self::push_part(&mut parts, &body[start..]);
        parts
    }

    fn push_part(parts: &mut Vec<String>, text: &str) {
        if !text.trim().is_empty() {
            parts.push(text.trim().to_owned());
        }
    }

    fn body_after(extension: &str, code: &str, at: usize) -> Option<(usize, usize)> {
        let open = at + code[at..].find('{')?;
        Self::closing(extension, code, open, b'{', b'}').map(|close| (open + 1, close))
    }

    fn parenthesized_after(extension: &str, code: &str, at: usize) -> Option<(usize, usize)> {
        let open = at + code[at..].find('(')?;
        Self::closing(extension, code, open, b'(', b')').map(|close| (open + 1, close))
    }

    fn closing(
        extension: &str,
        code: &str,
        open: usize,
        opening: u8,
        closing: u8,
    ) -> Option<usize> {
        let bytes = code.as_bytes();
        let mut quotes = Quotes::default();
        let mut depth = 0_i32;
        for at in open..bytes.len() {
            if quotes.inside(extension, bytes, at) {
                continue;
            }
            if bytes[at] == opening {
                depth += 1;
            } else if bytes[at] == closing {
                depth -= 1;
                if depth == 0 {
                    return Some(at);
                }
            }
        }
        None
    }

    fn identifier(text: &str) -> &str {
        let text = text.trim_start();
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        &text[..length]
    }

    fn base_name(text: &str) -> String {
        let text = text.trim().trim_start_matches(['*', '&']);
        let end = text.find(['<', '[', '(']).unwrap_or(text.len());
        let text = text[..end].trim();
        let after_colons = text.rsplit("::").next().unwrap_or(text);
        let after_dots = after_colons.rsplit('.').next().unwrap_or(after_colons);
        after_dots
            .rsplit('\\')
            .next()
            .unwrap_or(after_dots)
            .to_owned()
    }

    fn attributes(text: &str) -> (Vec<&str>, &str) {
        let mut found = Vec::new();
        let mut rest = text.trim_start();
        loop {
            let length = if let Some(after) = rest.strip_prefix('@') {
                let name = after
                    .bytes()
                    .take_while(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b':')
                    })
                    .count();
                let arguments = if after[name..].starts_with('(') {
                    Self::group_length(&after[name..], b'(', b')')
                } else {
                    Some(0)
                };
                arguments.map(|arguments| 1 + name + arguments)
            } else if rest.starts_with("#[") {
                Self::group_length(&rest[1..], b'[', b']').map(|group| 1 + group)
            } else if rest.starts_with('[') {
                Self::group_length(rest, b'[', b']')
            } else {
                None
            };
            let Some(length) = length.filter(|length| *length > 1) else {
                return (found, rest);
            };
            found.push(&rest[..length]);
            rest = rest[length..].trim_start();
        }
    }

    fn group_length(text: &str, opening: u8, closing: u8) -> Option<usize> {
        let bytes = text.as_bytes();
        let mut quotes = Quotes::default();
        let mut depth = 0_i32;
        for at in 0..bytes.len() {
            if quotes.inside("", bytes, at) {
                continue;
            }
            if bytes[at] == opening {
                depth += 1;
            } else if bytes[at] == closing {
                depth -= 1;
                if depth == 0 {
                    return Some(at + 1);
                }
            }
        }
        None
    }

    fn camel(name: &str) -> String {
        let mut characters = name.chars();
        let first = characters.next().unwrap_or_default().to_ascii_lowercase();
        format!("{first}{}", characters.as_str())
    }
}

struct TypeScriptModels;

impl TypeScriptModels {
    fn read(code: &str) -> Vec<(String, Model)> {
        let mut models = Vec::new();
        for (at, line) in Lines::starts(code) {
            let text = line
                .trim_start_matches("export ")
                .trim_start_matches("default ")
                .trim_start_matches("declare ");
            if let Some(rest) = text.strip_prefix("interface ") {
                let name = Members::identifier(rest);
                if name.is_empty() {
                    continue;
                }
                let Some((start, end)) = Members::body_after("ts", code, at) else {
                    continue;
                };
                let mut record = Self::record(&code[start..end]);
                record.bases = Self::extended(&rest[name.len()..]);
                models.push((name.to_owned(), Model::Record(record)));
                continue;
            }
            if let Some(rest) = text.strip_prefix("type ")
                && let Some(model) = Self::alias(code, at, rest)
            {
                models.push(model);
            }
        }
        models
    }

    fn extended(header: &str) -> Vec<String> {
        let header = header.trim_start();
        let header = match header.strip_prefix('<') {
            Some(_) => {
                Members::group_length(header, b'<', b'>').map_or("", |length| &header[length..])
            }
            None => header,
        };
        let header = header.split('{').next().unwrap_or("");
        let Some(list) = header.trim_start().strip_prefix("extends ") else {
            return Vec::new();
        };
        TypeText::split_top_level(list, ',')
            .iter()
            .map(|base| Members::base_name(base))
            .filter(|base| !base.is_empty())
            .collect()
    }

    fn alias(code: &str, at: usize, rest: &str) -> Option<(String, Model)> {
        let (name, _) = rest.split_once('=')?;
        let name = name.split('<').next().unwrap_or(name).trim();
        if !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return None;
        }
        let equals = at + code[at..].find('=')?;
        let statement = Self::statement(code, equals + 1);
        let parts = TypeText::split_top_level(statement, '&');
        let braced = statement.starts_with('{') && statement.ends_with('}');
        if parts.len() == 1 && !braced {
            let shape = TypeText::shape("ts", statement).unwrap_or(Shape::Any);
            return Some((name.to_owned(), Model::Alias(shape)));
        }
        let mut record = Record::default();
        for part in &parts {
            match part
                .strip_prefix('{')
                .and_then(|inner| inner.strip_suffix('}'))
            {
                Some(inner) => record.extend(Self::record(inner)),
                None => record.bases.push(Members::base_name(part)),
            }
        }
        Some((name.to_owned(), Model::Record(record)))
    }

    fn statement(code: &str, from: usize) -> &str {
        let bytes = code.as_bytes();
        let start = from + (code[from..].len() - code[from..].trim_start().len());
        let mut quotes = Quotes::default();
        let mut depth = 0_i32;
        for at in start..bytes.len() {
            if quotes.inside("ts", bytes, at) {
                continue;
            }
            depth += Members::depth_change(bytes, at);
            let ends = depth == 0
                && (bytes[at] == b';'
                    || (bytes[at] == b'\n' && !code[at..].trim_start().starts_with(['&', '|'])));
            if ends {
                return code[start..at].trim();
            }
        }
        code[start..].trim()
    }

    fn record(body: &str) -> Record {
        let mut record = Record::default();
        for member in Members::split("ts", body) {
            let member = member.trim_start_matches("readonly ").trim();
            if member.starts_with('[') {
                record.open = true;
                continue;
            }
            let Some((name, kind)) = member.split_once(':') else {
                continue;
            };
            if name.contains('(') {
                continue;
            }
            let optional = name.trim().ends_with('?');
            let name = name.trim().trim_end_matches('?').trim_matches(['\'', '"']);
            record.add("ts", name, kind, !optional);
        }
        record
    }
}

struct JsDocModels;

impl JsDocModels {
    fn typedef(block: &str) -> Option<(String, Model)> {
        let lines: Vec<&str> = block
            .lines()
            .map(|line| line.trim().trim_start_matches('*').trim())
            .collect();
        let typedef = lines
            .iter()
            .find_map(|line| line.strip_prefix("@typedef"))?;
        let (kind, name) = Self::braced(typedef)
            .unwrap_or_else(|| (String::from("Object"), typedef.trim().to_owned()));
        let name = name.split_whitespace().next()?.to_owned();
        if !matches!(kind.as_str(), "Object" | "object") {
            let shape = TypeText::shape("js", &kind).unwrap_or(Shape::Any);
            return Some((name, Model::Alias(shape)));
        }
        let mut record = Record::default();
        for line in &lines {
            let Some(property) = line
                .strip_prefix("@property")
                .or_else(|| line.strip_prefix("@prop"))
            else {
                continue;
            };
            let Some((kind, rest)) = Self::braced(property) else {
                continue;
            };
            let written = rest.split_whitespace().next().unwrap_or("");
            let optional = written.starts_with('[');
            let field_name = written
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split('=')
                .next()
                .unwrap_or("");
            if field_name.contains('.') {
                continue;
            }
            record.add("js", field_name, &kind, !optional);
        }
        Some((name, Model::Record(record)))
    }

    fn braced(text: &str) -> Option<(String, String)> {
        let text = text.trim();
        let rest = text.strip_prefix('{')?;
        let mut depth = 1_i32;
        for (offset, character) in rest.char_indices() {
            match character {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        let kind = rest[..offset].trim().to_owned();
                        let after = rest[offset + 1..].trim().to_owned();
                        return Some((kind, after));
                    }
                }
                _ => {}
            }
        }
        None
    }
}

struct Lines;

impl Lines {
    fn starts(code: &str) -> Vec<(usize, &str)> {
        let mut lines = Vec::new();
        let mut offset = 0;
        for line in code.split_inclusive('\n') {
            let trimmed = line.trim();
            let indent = line.len() - line.trim_start().len();
            lines.push((offset + indent, trimmed));
            offset += line.len();
        }
        lines
    }
}

struct PythonModels;

impl PythonModels {
    const PLAIN_BASES: [&'static str; 8] = [
        "object",
        "BaseModel",
        "TypedDict",
        "NamedTuple",
        "Protocol",
        "Generic",
        "ABC",
        "dataclass",
    ];

    fn read(code: &str) -> Vec<(String, Model)> {
        let lines: Vec<&str> = code.lines().collect();
        let mut models = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let Some(header) = line.strip_prefix("class ") else {
                continue;
            };
            let name: String = header
                .chars()
                .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
                .collect();
            let bases = header.split_once('(').map_or("", |(_, rest)| rest);
            let bases = bases.rsplit_once(')').map_or(bases, |(inside, _)| inside);
            let body = Self::body(&lines, index + 1);
            models.push((name, Self::model(bases, &body)));
        }
        models
    }

    fn body<'a>(lines: &[&'a str], start: usize) -> Vec<&'a str> {
        let mut body = Vec::new();
        for line in &lines[start..] {
            if !line.trim().is_empty() && !line.starts_with([' ', '\t']) {
                break;
            }
            body.push(line.trim());
        }
        body
    }

    fn model(bases: &str, body: &[&str]) -> Model {
        if bases.contains("Enum") {
            return Model::Alias(Self::enum_shape(body));
        }
        let total = !bases.contains("total=False");
        let mut record = Record {
            bases: Self::bases(bases),
            ..Record::default()
        };
        for line in body {
            let Some((name, rest)) = line.split_once(':') else {
                continue;
            };
            let name = name.trim();
            let valid = name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
            if !valid || name.is_empty() || name.starts_with("def") {
                continue;
            }
            let (kind, default) = match rest.split_once('=') {
                Some((kind, _)) => (kind.trim(), true),
                None => (rest.trim(), false),
            };
            let (kind, not_required) = match kind
                .strip_prefix("NotRequired[")
                .and_then(|inner| inner.strip_suffix(']'))
            {
                Some(inner) => (inner, true),
                None => (kind, false),
            };
            let required = total && !default && !not_required;
            record.add("py", name, kind, required);
        }
        Model::Record(record)
    }

    fn bases(text: &str) -> Vec<String> {
        TypeText::split_top_level(text, ',')
            .iter()
            .filter(|base| !base.contains('=') && !base.trim().is_empty())
            .map(|base| Members::base_name(base))
            .filter(|base| !Self::PLAIN_BASES.contains(&base.as_str()))
            .collect()
    }

    fn enum_shape(body: &[&str]) -> Shape {
        for line in body {
            let Some((_, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();
            if value.starts_with(['"', '\'']) {
                return Shape::Text;
            }
            if value.parse::<i64>().is_ok() {
                return Shape::Integer;
            }
        }
        Shape::Any
    }
}

struct GoModels;

impl GoModels {
    fn read(code: &str) -> Vec<(String, Model)> {
        let mut models = Vec::new();
        for (at, line) in Lines::starts(code) {
            let Some(rest) = line.strip_prefix("type ") else {
                continue;
            };
            let Some(name) = rest.split_whitespace().next() else {
                continue;
            };
            let name = name.split('[').next().unwrap_or(name);
            let kind = Self::after_type_parameters(&rest[name.len()..]);
            if kind.starts_with("struct") {
                let Some((start, end)) = Members::body_after("go", code, at) else {
                    continue;
                };
                models.push((
                    name.to_owned(),
                    Model::Record(Self::record(&code[start..end])),
                ));
                continue;
            }
            let kind = kind.trim_start_matches('=').trim();
            let shape = TypeText::shape("go", kind).unwrap_or(Shape::Any);
            models.push((name.to_owned(), Model::Alias(shape)));
        }
        models
    }

    fn after_type_parameters(text: &str) -> &str {
        let text = text.trim_start();
        let parameters = text
            .starts_with('[')
            .then(|| Members::group_length(text, b'[', b']'))
            .flatten()
            .unwrap_or(0);
        text[parameters..].trim_start()
    }

    fn record(body: &str) -> Record {
        let mut record = Record::default();
        for line in body.lines() {
            let line = line.trim();
            let (declaration, tag) = match line.find('`') {
                Some(at) => (line[..at].trim(), &line[at..]),
                None => (line, ""),
            };
            if declaration.is_empty() {
                continue;
            }
            if !declaration.contains([' ', '\t', ',']) {
                Self::embedded(&mut record, declaration, tag);
                continue;
            }
            let (names, kind) = Self::names_and_kind(declaration);
            for name in names {
                if !name.starts_with(|character: char| character.is_ascii_uppercase()) {
                    continue;
                }
                let (json_name, omit_empty) = Self::json_name(name, tag);
                if json_name != "-" {
                    record.add("go", &json_name, kind, !omit_empty);
                }
            }
        }
        record
    }

    fn embedded(record: &mut Record, kind: &str, tag: &str) {
        let written = Members::base_name(kind);
        let (json_name, omit_empty) = Self::json_name(&written, tag);
        if json_name == "-" {
            return;
        }
        if json_name == written {
            record.bases.push(written);
            return;
        }
        record.add("go", &json_name, kind, !omit_empty);
    }

    fn names_and_kind(declaration: &str) -> (Vec<&str>, &str) {
        let mut names = Vec::new();
        let mut rest = declaration.trim_start();
        loop {
            let name = Members::identifier(rest);
            if name.is_empty() {
                return (names, rest);
            }
            names.push(name);
            rest = rest[name.len()..].trim_start();
            match rest.strip_prefix(',') {
                Some(after) => rest = after.trim_start(),
                None => return (names, rest),
            }
        }
    }

    fn json_name(name: &str, tag: &str) -> (String, bool) {
        let Some(start) = tag.find("json:\"") else {
            return (name.to_owned(), false);
        };
        let rest = &tag[start + 6..];
        let value = rest.split('"').next().unwrap_or("");
        let mut parts = value.split(',');
        let written = parts.next().unwrap_or("");
        let omit_empty = parts.any(|part| part == "omitempty");
        if written.is_empty() {
            return (name.to_owned(), omit_empty);
        }
        (written.to_owned(), omit_empty)
    }
}

struct Declaration<'a> {
    kind: &'static str,
    name: &'a str,
    offset: usize,
}

struct BracedModels;

impl BracedModels {
    fn read(extension: &str, code: &str) -> Vec<(String, Model)> {
        let mut models = Vec::new();
        for (at, line) in Lines::starts(code) {
            let Some(declaration) = Self::declaration(line) else {
                continue;
            };
            let model = match declaration.kind {
                "enum" => Model::Alias(Self::enum_shape(extension, line)),
                _ => Model::Record(Self::record(extension, code, at, &declaration)),
            };
            models.push((declaration.name.to_owned(), model));
        }
        models
    }

    fn declaration(line: &str) -> Option<Declaration<'_>> {
        let mut words = Vec::new();
        let mut start = None;
        for (offset, character) in line.char_indices() {
            let separator =
                character.is_whitespace() || matches!(character, '(' | '{' | '<' | ':' | ';');
            match (separator, start) {
                (true, Some(begin)) => {
                    words.push((begin, &line[begin..offset]));
                    start = None;
                }
                (false, None) => start = Some(offset),
                _ => {}
            }
        }
        if let Some(begin) = start {
            words.push((begin, &line[begin..]));
        }
        for (index, (offset, word)) in words.iter().enumerate() {
            let kind = match *word {
                "class" => "class",
                "record" => "record",
                "struct" => "struct",
                "enum" => "enum",
                _ => continue,
            };
            let mut name = words.get(index + 1)?.1;
            if kind == "record" && ["struct", "class"].contains(&name) {
                name = words.get(index + 2)?.1;
            }
            if name == "class" {
                let name = words.get(index + 2)?.1;
                return Some(Declaration {
                    kind: "enum",
                    name,
                    offset: *offset,
                });
            }
            return Some(Declaration {
                kind,
                name,
                offset: *offset,
            });
        }
        None
    }

    fn enum_shape(extension: &str, line: &str) -> Shape {
        if extension == "php" && line.contains(": int") {
            return Shape::Integer;
        }
        Shape::Text
    }

    fn record(extension: &str, code: &str, at: usize, declaration: &Declaration<'_>) -> Record {
        let mut record = Record::default();
        let keyword_end = at + declaration.offset + declaration.kind.len();
        let name_end = keyword_end
            + code[keyword_end..].find(declaration.name).unwrap_or(0)
            + declaration.name.len();
        let mut after_header = name_end;
        let parameters = code[name_end..]
            .find(['{', ';', '('])
            .map(|stop| code.as_bytes()[name_end + stop])
            == Some(b'(');
        if parameters
            && let Some((open, close)) = Members::parenthesized_after(extension, code, name_end)
        {
            Self::parameters(extension, &code[open..close], &mut record);
            after_header = close + 1;
        }
        let header_end = after_header
            + code[after_header..]
                .find(['{', ';'])
                .unwrap_or(code.len() - after_header);
        let tail = &code[after_header..header_end];
        let first_line = tail.trim_start().lines().next().unwrap_or("");
        record.bases = Self::bases(extension, first_line);
        let another_type = tail
            .lines()
            .any(|line| Self::declaration(line.trim()).is_some());
        let has_body = code.as_bytes().get(header_end) == Some(&b'{') && !another_type;
        if has_body && let Some((open, close)) = Members::body_after(extension, code, header_end) {
            Self::members(extension, &code[open..close], &mut record);
        }
        record
    }

    fn bases(extension: &str, header: &str) -> Vec<String> {
        let header = header.split(" where ").next().unwrap_or("");
        let list = match extension {
            "kt" | "cs" => header.trim_start().strip_prefix(':').unwrap_or(""),
            _ => Self::after_word(header, "extends"),
        };
        let mut bases: Vec<String> = TypeText::split_top_level(list, ',')
            .iter()
            .filter(|base| !base.is_empty())
            .filter(|base| extension != "kt" || base.contains('('))
            .map(|base| Members::base_name(base))
            .filter(|base| extension != "cs" || !Self::csharp_interface(base))
            .collect();
        if extension == "dart" {
            let mixins = Self::after_word(header, "with");
            bases.extend(
                TypeText::split_top_level(mixins, ',')
                    .iter()
                    .filter(|base| !base.is_empty())
                    .map(|base| Members::base_name(base)),
            );
        }
        bases
    }

    fn after_word<'a>(header: &'a str, word: &str) -> &'a str {
        let marker = format!("{word} ");
        let Some(at) = header
            .match_indices(&marker)
            .map(|(at, _)| at)
            .find(|at| *at == 0 || header.as_bytes()[at - 1].is_ascii_whitespace())
        else {
            return "";
        };
        let rest = &header[at + marker.len()..];
        let end = ["implements ", "with ", "extends ", "{"]
            .iter()
            .filter_map(|stop| rest.find(stop))
            .min()
            .unwrap_or(rest.len());
        &rest[..end]
    }

    fn csharp_interface(name: &str) -> bool {
        let bytes = name.as_bytes();
        bytes.len() > 1 && bytes[0] == b'I' && bytes[1].is_ascii_uppercase()
    }

    fn parameters(extension: &str, list: &str, record: &mut Record) {
        for parameter in Members::split(extension, list) {
            let (attributes, parameter) = Members::attributes(&parameter);
            let (declaration, default) = match parameter.split_once('=') {
                Some((declaration, _)) => (declaration.trim(), true),
                None => (parameter, false),
            };
            let Some((name, kind)) = Self::name_and_type(extension, declaration) else {
                continue;
            };
            let name = Self::written_name(&attributes).unwrap_or(name);
            record.add(extension, &name, &kind, !default);
        }
    }

    fn members(extension: &str, body: &str, record: &mut Record) {
        let bytes = body.as_bytes();
        let mut quotes = Quotes::default();
        let mut depth = 0_i32;
        let mut start = 0;
        let mut pending = Vec::new();
        let mut constructor = None;
        for at in 0..bytes.len() {
            if quotes.inside(extension, bytes, at) {
                continue;
            }
            match bytes[at] {
                b'{' | b'(' => depth += 1,
                b'}' | b')' => depth -= 1,
                _ => {}
            }
            if depth == 0 && matches!(bytes[at], b';' | b'\n' | b'}') {
                let end = if bytes[at] == b'}' { at + 1 } else { at };
                Self::add_member(
                    extension,
                    body[start..end].trim(),
                    record,
                    &mut pending,
                    &mut constructor,
                );
                start = at + 1;
            }
        }
        Self::add_member(
            extension,
            body[start..].trim(),
            record,
            &mut pending,
            &mut constructor,
        );
        if let Some(promoted) = constructor {
            for field in &mut record.fields {
                if !promoted.contains(&field.name) {
                    field.required = false;
                }
            }
        }
    }

    fn add_member(
        extension: &str,
        text: &str,
        record: &mut Record,
        pending: &mut Vec<String>,
        constructor: &mut Option<Vec<String>>,
    ) {
        if extension == "php"
            && let Some(at) = text.find("function __construct")
        {
            if let Some((start, end)) = Members::parenthesized_after(extension, text, at) {
                *constructor = Some(PhpParameters::read(&text[start..end], record));
            }
            pending.clear();
            return;
        }
        if extension == "php" && text.starts_with("use ") {
            record
                .bases
                .extend(text[4..].split(',').map(Members::base_name));
            return;
        }
        let (attributes, rest) = Members::attributes(text);
        pending.extend(attributes.iter().map(|attribute| (*attribute).to_owned()));
        if rest.trim().is_empty() {
            return;
        }
        let written = std::mem::take(pending);
        let written: Vec<&str> = written.iter().map(String::as_str).collect();
        if let Some((name, kind, required)) = Self::member(extension, rest) {
            let name = Self::written_name(&written).unwrap_or(name);
            record.add(extension, &name, &kind, required);
        }
    }

    fn written_name(attributes: &[&str]) -> Option<String> {
        attributes.iter().find_map(|attribute| {
            let inside = attribute
                .trim_start_matches('[')
                .trim_end_matches(']')
                .trim()
                .strip_prefix("JsonPropertyName(")?;
            let name = inside.trim().strip_prefix('"')?.split('"').next()?;
            Some(name.to_owned())
        })
    }

    fn member(extension: &str, text: &str) -> Option<(String, String, bool)> {
        let text = text.trim().trim_end_matches(',');
        let hidden = extension == "php"
            && Self::php_modifiers(text)
                .iter()
                .any(|modifier| matches!(modifier.as_str(), "private" | "protected"));
        let text = &Self::without_set_visibility(text);
        let skipped = text.is_empty()
            || hidden
            || text.contains(" static ")
            || text.starts_with("static ")
            || text.contains("function ")
            || text.contains("fun ")
            || (text.contains('(') && !text.contains("{ get"));
        if skipped {
            return None;
        }
        let property = text.contains("{ get");
        let declaration = match text.find("{ get") {
            Some(at) => &text[..at],
            None => text,
        };
        let (declaration, initialized) = match declaration.split_once('=') {
            Some((declaration, _)) => (declaration.trim(), true),
            None => (declaration.trim(), false),
        };
        let initialized = initialized || (property && text.contains("} ="));
        let (name, kind) = Self::name_and_type(extension, declaration)?;
        Some((name, kind, !initialized))
    }

    fn without_set_visibility(text: &str) -> String {
        let mut text = text.to_owned();
        for written in ["public(set)", "protected(set)", "private(set)"] {
            text = text.replace(written, "");
        }
        text
    }

    fn php_modifiers(text: &str) -> Vec<String> {
        let mut modifiers = Vec::new();
        let mut rest = text.trim_start();
        loop {
            let word = Members::identifier(rest);
            let lower = word.to_ascii_lowercase();
            if !matches!(
                lower.as_str(),
                "public" | "protected" | "private" | "readonly" | "final"
            ) {
                return modifiers;
            }
            rest = &rest[word.len()..];
            match rest.strip_prefix("(set)") {
                Some(after) => {
                    rest = after;
                    modifiers.push(String::from("set"));
                }
                None => modifiers.push(lower),
            }
            rest = rest.trim_start();
        }
    }

    fn name_and_type(extension: &str, declaration: &str) -> Option<(String, String)> {
        let mut text = declaration.trim().to_owned();
        while let Some(rest) = Self::without_modifier(extension, &text) {
            text = rest;
        }
        if text.is_empty() {
            return None;
        }
        Self::split_name_and_type(extension, &text)
    }

    fn without_modifier(extension: &str, text: &str) -> Option<String> {
        [
            "public ",
            "private ",
            "protected ",
            "internal ",
            "readonly ",
            "final ",
            "late ",
            "required ",
            "pub(crate) ",
            "pub ",
            "val ",
            "var ",
            "let ",
            "const ",
            "override ",
            "open ",
        ]
        .iter()
        .find(|modifier| {
            text.get(..modifier.len()).is_some_and(|start| {
                start == **modifier || (extension == "php" && start.eq_ignore_ascii_case(modifier))
            })
        })
        .map(|modifier| text[modifier.len()..].trim_start().to_owned())
    }

    fn split_name_and_type(extension: &str, text: &str) -> Option<(String, String)> {
        match extension {
            "kt" | "rs" | "swift" => {
                let (name, kind) = text.split_once(':')?;
                Some((name.trim().to_owned(), kind.trim().to_owned()))
            }
            "php" => {
                let at = text.find('$')?;
                let name = text[at + 1..].trim().to_owned();
                let kind = text[..at].trim().trim_end_matches(['&', '.']).trim();
                Some((name, kind.to_owned()))
            }
            "cs" => {
                let (kind, name) = text.rsplit_once(char::is_whitespace)?;
                Some((Members::camel(name.trim()), kind.trim().to_owned()))
            }
            _ => {
                let (kind, name) = text.rsplit_once(char::is_whitespace)?;
                let name = name.trim().trim_start_matches("this.");
                Some((name.to_owned(), kind.trim().to_owned()))
            }
        }
    }
}

struct PhpParameters;

impl PhpParameters {
    fn read(list: &str, record: &mut Record) -> Vec<String> {
        let mut promoted = Vec::new();
        for parameter in Members::split("php", list) {
            let (_, parameter) = Members::attributes(&parameter);
            let modifiers = BracedModels::php_modifiers(parameter);
            let visible = modifiers.iter().any(|modifier| modifier == "public")
                || !modifiers
                    .iter()
                    .any(|modifier| matches!(modifier.as_str(), "private" | "protected"));
            let (declaration, default) = match parameter.split_once('=') {
                Some((declaration, _)) => (declaration.trim(), true),
                None => (parameter, false),
            };
            let declaration = BracedModels::without_set_visibility(declaration);
            let Some((name, kind)) = BracedModels::name_and_type("php", &declaration) else {
                continue;
            };
            if !modifiers.is_empty() && visible {
                record.add("php", &name, &kind, !default);
                promoted.push(name);
            } else if !default && record.unreadable.is_none() {
                record.unreadable = Some(format!(
                    "the constructor needs ${name}, but ${name} is not a public property, so a request body cannot fill it. Make it public (public {kind} ${name}) or give it a default value"
                ));
            }
        }
        promoted
    }
}
