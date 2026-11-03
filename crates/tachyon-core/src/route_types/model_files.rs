use super::{Model, ModelField, Record, Shape, TypeText};
use crate::ascii_name::AsciiName;
use std::ops::Range;
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
    const MAX_FILES: usize = 1024;
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
        Self::require_shallow(portable, depth)?;
        for path in Self::sorted_paths(folder) {
            Self::read_entry(&path, portable, extension, depth, files, total)?;
        }
        Ok(())
    }

    fn require_shallow(portable: &Path, depth: usize) -> Result<(), String> {
        (depth <= Self::MAX_DEPTH)
            .then_some(())
            .ok_or_else(|| format!("{} is nested more than 8 folders deep", Self::top(portable)))
    }

    fn sorted_paths(folder: &Path) -> Vec<PathBuf> {
        let entries = std::fs::read_dir(folder).ok();
        let mut paths: Vec<PathBuf> = entries
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect();
        paths.sort();
        paths
    }

    fn read_entry(
        path: &Path,
        portable: &Path,
        extension: &str,
        depth: usize,
        files: &mut Vec<(String, String)>,
        total: &mut usize,
    ) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(path).ok();
        let child = portable.join(path.file_name().unwrap_or_default());
        match metadata {
            Some(metadata) if metadata.is_dir() => {
                Self::read_directory(path, &child, extension, depth + 1, files, total)
            }
            Some(metadata) if metadata.is_file() && Self::reads(extension, path) => {
                Self::read_file(path, portable, &child, files, total)
            }
            _ => Ok(()),
        }
    }

    fn read_file(
        path: &Path,
        portable: &Path,
        child: &Path,
        files: &mut Vec<(String, String)>,
        total: &mut usize,
    ) -> Result<(), String> {
        let source = std::fs::read_to_string(path)
            .map_err(|_| format!("Cannot read {}", child.display()))?;
        *total += source.len();
        let within_limits = [
            files.len() < Self::MAX_FILES,
            *total <= Self::MAX_TOTAL_BYTES,
        ] == [true, true];
        within_limits.then_some(()).ok_or_else(|| {
            format!(
                "{} holds more than 1024 files or 1 MiB",
                Self::top(portable)
            )
        })?;
        files.push((child.to_string_lossy().replace('\\', "/"), source));
        Ok(())
    }

    fn reads(extension: &str, path: &Path) -> bool {
        let found = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        [
            found == extension,
            Self::ALSO_READS.contains(&(extension, found)),
        ]
        .contains(&true)
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
    const TEXT_QUOTES: &'static [u8] = b"\"'`";
    const LINE_BREAKS: &'static [u8] = b"\n\r";

    pub(super) fn models(extension: &str, source: &str) -> Vec<(String, Model)> {
        Self::shadowed_models(extension, source)
            .unwrap_or_else(|| Self::own_models(extension, source))
    }

    fn shadowed_models(extension: &str, source: &str) -> Option<Vec<(String, Model)>> {
        if extension == crate::fsharp_source::FsharpSource::EXTENSION {
            Some(Self::models(
                "cs",
                crate::fsharp_source::FsharpShadow::of(source).text(),
            ))
        } else if extension == crate::lua_source::LuaSource::EXTENSION {
            Some(Self::models(
                "rb",
                crate::lua_source::LuaShadow::joined(source).text(),
            ))
        } else if extension == crate::tac_cpp::CppSource::EXTENSION {
            Some(crate::yon_cpp::CppYonModels::read(source))
        } else {
            None
        }
    }

    fn own_models(extension: &str, source: &str) -> Vec<(String, Model)> {
        let typed = Self::with_typed_arrays(extension, source);
        let source = typed.as_str();
        let code = Self::without_comments(extension, source);
        let syntax = crate::groovy_source::GroovySource::syntax(extension);
        Self::language_models(syntax, source, &code)
    }

    fn with_typed_arrays(extension: &str, source: &str) -> String {
        let typed = (extension == "php").then(|| super::PhpDoc::typed_arrays(source));
        typed.unwrap_or_else(|| source.to_owned())
    }

    fn language_models(extension: &str, source: &str, code: &str) -> Vec<(String, Model)> {
        match extension {
            "ts" => TypeScriptModels::read(code),
            "js" => Self::jsdoc_models(source),
            "rb" => ruby::RubyModels::read(source),
            "py" => PythonModels::read(code),
            "go" => GoModels::read(code),
            "rs" => [
                rust_fields::RustFields::models(code, BracedModels::read(extension, code)),
                alias_models::AliasModels::read(extension, code),
            ]
            .concat(),
            "java" | "kt" | "cs" | "dart" | "php" | "swift" => [
                BracedModels::read(extension, code),
                alias_models::AliasModels::read(extension, code),
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
        Self::jsdoc_blocks(source)
            .into_iter()
            .filter_map(JsDocModels::typedef)
            .collect()
    }

    fn jsdoc_blocks(source: &str) -> Vec<&str> {
        let mut blocks = Vec::new();
        let mut rest = source;
        while let Some((block, after)) = Self::next_jsdoc_block(rest) {
            blocks.push(block);
            rest = after;
        }
        blocks
    }

    fn next_jsdoc_block(rest: &str) -> Option<(&str, &str)> {
        let start = rest.find("/**")?;
        let close = start + 2 + rest[start + 2..].find("*/")?;
        let body = rest.get(start + 3..close).unwrap_or_default();
        Some((body, &rest[close + 2..]))
    }

    fn without_comments(extension: &str, source: &str) -> String {
        let mask = crate::lexical::code_mask(extension, source);
        let bytes = source.as_bytes();
        let visible: Vec<bool> = (0..bytes.len())
            .map(|at| mask.code.get(at).copied().unwrap_or(true))
            .collect();
        let mut output = bytes.to_vec();
        let comments = Self::runs(&visible)
            .into_iter()
            .filter(|(is_visible, _)| !is_visible)
            .filter(|(_, run)| !Self::TEXT_QUOTES.contains(&bytes[run.start]));
        for (_, run) in comments {
            Self::blank(&mut output[run]);
        }
        String::from_utf8(output).unwrap_or_default()
    }

    fn runs(flags: &[bool]) -> Vec<(bool, Range<usize>)> {
        let mut start = 0;
        flags
            .chunk_by(|left, right| left == right)
            .map(|chunk| {
                let run = start..start + chunk.len();
                start = run.end;
                (chunk[0], run)
            })
            .collect()
    }

    fn blank(bytes: &mut [u8]) {
        for byte in bytes {
            *byte = Self::blanked(*byte);
        }
    }

    fn blanked(byte: u8) -> u8 {
        if Self::LINE_BREAKS.contains(&byte) {
            byte
        } else {
            b' '
        }
    }
}

impl Record {
    fn add(&mut self, extension: &str, name: &str, type_text: &str, required: bool) {
        let shape = TypeText::shape(extension, type_text).ok();
        self.open |= shape.is_none();
        self.fields
            .extend(shape.map(|shape| Self::field(name, shape, required)));
    }

    fn add_each(
        &mut self,
        extension: &str,
        fields: impl IntoIterator<Item = (String, String, bool)>,
    ) {
        for (name, type_text, required) in fields {
            self.add(extension, &name, &type_text, required);
        }
    }

    fn field(name: &str, shape: Shape, required: bool) -> ModelField {
        let required = required && !shape.is_optional();
        ModelField {
            name: name.to_owned(),
            shape,
            required,
        }
    }

    fn extend(&mut self, other: Self) {
        self.fields.extend(other.fields);
        self.bases.extend(other.bases);
        self.open |= other.open;
        self.unreadable = self.unreadable.take().or(other.unreadable);
    }
}

#[derive(Default)]
struct Quotes {
    open: Option<u8>,
    escaped: bool,
}

impl Quotes {
    fn inside(&mut self, extension: &str, bytes: &[u8], at: usize) -> bool {
        let continued = self.open.map(|open| self.continue_quote(open, bytes[at]));
        continued.unwrap_or_else(|| self.open_quote(extension, bytes, at))
    }

    fn continue_quote(&mut self, open: u8, byte: u8) -> bool {
        if self.escaped {
            self.escaped = false;
        } else if byte == b'\\' && open != b'`' {
            self.escaped = true;
        } else if byte == open {
            self.open = None;
        }
        true
    }

    fn open_quote(&mut self, extension: &str, bytes: &[u8], at: usize) -> bool {
        let starts = Self::starts_quote(extension, bytes, at);
        self.open = starts.then_some(bytes[at]);
        starts
    }

    fn starts_quote(extension: &str, bytes: &[u8], at: usize) -> bool {
        match bytes[at] {
            b'\'' if extension == "rs" => Self::rust_character(bytes, at),
            b'"' | b'`' | b'\'' => true,
            _ => false,
        }
    }

    fn rust_character(bytes: &[u8], at: usize) -> bool {
        [
            bytes.get(at + 1) == Some(&b'\\'),
            bytes.get(at + 2) == Some(&b'\''),
        ]
        .contains(&true)
    }
}

struct Members;

impl Members {
    const ARROW_STARTS: &'static [u8] = b"=-";
    const PART_ENDS: &'static [u8] = b";,\n";
    const ANNOTATION_SYMBOLS: &'static [u8] = b"_.:";

    fn depth_change(bytes: &[u8], at: usize) -> i32 {
        match bytes[at] {
            b'<' | b'[' | b'(' | b'{' => 1,
            b'>' if at
                .checked_sub(1)
                .is_some_and(|before| Self::ARROW_STARTS.contains(&bytes[before])) =>
            {
                0
            }
            b'>' | b']' | b')' | b'}' => -1,
            _ => 0,
        }
    }

    fn top_level_positions(
        extension: &str,
        bytes: &[u8],
        from: usize,
        depth_change: fn(&[u8], usize) -> i32,
    ) -> impl Iterator<Item = usize> {
        let mut quotes = Quotes::default();
        let mut depth = 0_i32;
        (from..bytes.len())
            .filter(move |&at| !quotes.inside(extension, bytes, at))
            .filter(move |&at| {
                depth += depth_change(bytes, at);
                depth == 0
            })
    }

    fn split(extension: &str, body: &str) -> Vec<String> {
        let bytes = body.as_bytes();
        let mut parts = Vec::new();
        let mut start = 0;
        let part_ends = Self::top_level_positions(extension, bytes, 0, Self::depth_change)
            .filter(|&at| Self::PART_ENDS.contains(&bytes[at]));
        for at in part_ends {
            Self::push_part(&mut parts, &body[start..at]);
            start = at + 1;
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
        (open..bytes.len())
            .filter(|&at| !quotes.inside(extension, bytes, at))
            .find(|&at| Self::closes(&mut depth, bytes[at], opening, closing))
    }

    fn closes(depth: &mut i32, byte: u8, opening: u8, closing: u8) -> bool {
        if byte == opening {
            *depth += 1;
            false
        } else if byte == closing {
            *depth -= 1;
            *depth == 0
        } else {
            false
        }
    }

    fn identifier(text: &str) -> &str {
        AsciiName::leading(text.trim_start())
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

    fn base_names(list: &str) -> Vec<String> {
        TypeText::split_top_level(list, ',')
            .iter()
            .map(|base| Self::base_name(base))
            .filter(|base| !base.is_empty())
            .collect()
    }

    fn attributes(text: &str) -> (Vec<&str>, &str) {
        let mut found = Vec::new();
        let mut rest = text.trim_start();
        while let Some(length) = Self::attribute_length(rest) {
            found.push(&rest[..length]);
            rest = rest[length..].trim_start();
        }
        (found, rest)
    }

    fn attribute_length(rest: &str) -> Option<usize> {
        let length = if let Some(after) = rest.strip_prefix('@') {
            Self::annotation_length(after)
        } else if rest.starts_with("#[") {
            Self::group_length(&rest[1..], b'[', b']').map(|group| 1 + group)
        } else if rest.starts_with('[') {
            Self::group_length(rest, b'[', b']')
        } else {
            None
        };
        length.filter(|length| *length > 1)
    }

    fn annotation_length(after: &str) -> Option<usize> {
        let name = after
            .bytes()
            .take_while(|&byte| Self::is_annotation_byte(byte))
            .count();
        let arguments = &after[name..];
        let grouped = arguments
            .starts_with('(')
            .then(|| Self::group_length(arguments, b'(', b')'));
        grouped
            .unwrap_or(Some(0))
            .map(|arguments| 1 + name + arguments)
    }

    fn is_annotation_byte(byte: u8) -> bool {
        [
            byte.is_ascii_alphanumeric(),
            Self::ANNOTATION_SYMBOLS.contains(&byte),
        ]
        .contains(&true)
    }

    fn is_word_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }

    fn group_length(text: &str, opening: u8, closing: u8) -> Option<usize> {
        Self::closing("", text, 0, opening, closing).map(|at| at + 1)
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
        Lines::models(code, Self::line_model)
    }

    fn line_model(code: &str, at: usize, line: &str) -> Option<(String, Model)> {
        let text = line
            .trim_start_matches("export ")
            .trim_start_matches("default ")
            .trim_start_matches("declare ");
        text.strip_prefix("interface ").map_or_else(
            || Self::type_alias(code, at, text),
            |rest| Self::interface(code, at, rest),
        )
    }

    fn interface(code: &str, at: usize, rest: &str) -> Option<(String, Model)> {
        let name = Some(Members::identifier(rest)).filter(|name| !name.is_empty())?;
        let (start, end) = Members::body_after("ts", code, at)?;
        let mut record = Self::record(&code[start..end]);
        record.bases = Self::extended(&rest[name.len()..]);
        Some((name.to_owned(), Model::Record(record)))
    }

    fn type_alias(code: &str, at: usize, text: &str) -> Option<(String, Model)> {
        let rest = text.strip_prefix("type ")?;
        Self::alias(code, at, rest)
    }

    fn extended(header: &str) -> Vec<String> {
        let header = Self::without_type_parameters(header.trim_start());
        let header = header.split('{').next().unwrap_or("");
        header
            .trim_start()
            .strip_prefix("extends ")
            .map_or_else(Vec::new, Members::base_names)
    }

    fn without_type_parameters(header: &str) -> &str {
        if !header.starts_with('<') {
            return header;
        }
        Members::group_length(header, b'<', b'>').map_or("", |length| &header[length..])
    }

    fn alias(code: &str, at: usize, rest: &str) -> Option<(String, Model)> {
        let (name, _) = rest.split_once('=')?;
        let name = name.split('<').next().unwrap_or(name).trim();
        let name = Some(name).filter(|name| name.bytes().all(Members::is_word_byte))?;
        let equals = at + code[at..].find('=')?;
        let statement = Self::statement(code, equals + 1);
        let parts = TypeText::split_top_level(statement, '&');
        Some((name.to_owned(), Self::alias_model(statement, &parts)))
    }

    fn alias_model(statement: &str, parts: &[String]) -> Model {
        let braced = [statement.starts_with('{'), statement.ends_with('}')] == [true, true];
        if parts.len() == 1 && !braced {
            return Model::Alias(TypeText::shape("ts", statement).unwrap_or(Shape::Any));
        }
        Model::Record(Self::intersection(parts))
    }

    fn intersection(parts: &[String]) -> Record {
        let mut record = Record::default();
        for part in parts {
            Self::add_part(&mut record, part);
        }
        record
    }

    fn add_part(record: &mut Record, part: &str) {
        match part
            .strip_prefix('{')
            .and_then(|inner| inner.strip_suffix('}'))
        {
            Some(inner) => record.extend(Self::record(inner)),
            None => record.bases.push(Members::base_name(part)),
        }
    }

    fn statement(code: &str, from: usize) -> &str {
        let start = from + (code[from..].len() - code[from..].trim_start().len());
        let end = Members::top_level_positions("ts", code.as_bytes(), start, Members::depth_change)
            .find(|&at| Self::ends_statement(code, at));
        end.map_or(&code[start..], |end| &code[start..end]).trim()
    }

    fn ends_statement(code: &str, at: usize) -> bool {
        match code.as_bytes()[at] {
            b';' => true,
            b'\n' => !code[at..].trim_start().starts_with(['&', '|']),
            _ => false,
        }
    }

    fn record(body: &str) -> Record {
        let members = Members::split("ts", body);
        let mut record = Record {
            open: members
                .iter()
                .any(|member| Self::is_index_signature(member)),
            ..Record::default()
        };
        let properties = members.iter().filter_map(|member| Self::property(member));
        record.add_each("ts", properties);
        record
    }

    fn without_readonly(member: &str) -> &str {
        member.trim_start_matches("readonly ").trim()
    }

    fn is_index_signature(member: &str) -> bool {
        Self::without_readonly(member).starts_with('[')
    }

    fn property(member: &str) -> Option<(String, String, bool)> {
        let member =
            Some(Self::without_readonly(member)).filter(|member| !member.starts_with('['))?;
        let (name, kind) = member.split_once(':')?;
        let name = Some(name).filter(|name| !name.contains('('))?;
        let optional = name.trim().ends_with('?');
        let name = name.trim().trim_end_matches('?').trim_matches(['\'', '"']);
        Some((name.to_owned(), kind.to_owned(), !optional))
    }
}

struct JsDocModels;

impl JsDocModels {
    const OBJECT_KINDS: [&'static str; 2] = ["Object", "object"];

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
        Some((name, Self::typedef_model(&kind, &lines)))
    }

    fn typedef_model(kind: &str, lines: &[&str]) -> Model {
        if !Self::OBJECT_KINDS.contains(&kind) {
            return Model::Alias(TypeText::shape("js", kind).unwrap_or(Shape::Any));
        }
        let mut record = Record::default();
        record.add_each("js", lines.iter().filter_map(|line| Self::property(line)));
        Model::Record(record)
    }

    fn property(line: &str) -> Option<(String, String, bool)> {
        let property = line
            .strip_prefix("@property")
            .or_else(|| line.strip_prefix("@prop"))?;
        let (kind, rest) = Self::braced(property)?;
        let written = rest.split_whitespace().next().unwrap_or("");
        let optional = written.starts_with('[');
        let field_name = written
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split('=')
            .next()
            .unwrap_or("");
        let field_name = Some(field_name).filter(|name| !name.contains('.'))?;
        Some((field_name.to_owned(), kind, !optional))
    }

    fn braced(text: &str) -> Option<(String, String)> {
        let rest = text.trim().strip_prefix('{')?;
        let mut depth = 1_i32;
        let close = rest
            .bytes()
            .position(|byte| Members::closes(&mut depth, byte, b'{', b'}'))?;
        let kind = rest[..close].trim().to_owned();
        let after = rest[close + 1..].trim().to_owned();
        Some((kind, after))
    }
}

struct Lines;

impl Lines {
    fn models(
        code: &str,
        line_model: fn(&str, usize, &str) -> Option<(String, Model)>,
    ) -> Vec<(String, Model)> {
        Self::starts(code)
            .into_iter()
            .filter_map(|(at, line)| line_model(code, at, line))
            .collect()
    }

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
        lines
            .iter()
            .enumerate()
            .filter_map(|(index, line)| {
                let header = line.strip_prefix("class ")?;
                Some(Self::class_model(&lines, index, header))
            })
            .collect()
    }

    fn class_model(lines: &[&str], index: usize, header: &str) -> (String, Model) {
        let name = AsciiName::leading(header).to_owned();
        let bases = header.split_once('(').map_or("", |(_, rest)| rest);
        let bases = bases.rsplit_once(')').map_or(bases, |(inside, _)| inside);
        let body = Self::body(lines, index + 1);
        (name, Self::model(bases, &body))
    }

    fn body<'a>(lines: &[&'a str], start: usize) -> Vec<&'a str> {
        lines[start..]
            .iter()
            .take_while(|line| Self::is_in_body(line))
            .map(|line| line.trim())
            .collect()
    }

    fn is_in_body(line: &str) -> bool {
        line.trim().is_empty() || line.starts_with([' ', '\t'])
    }

    fn model(bases: &str, body: &[&str]) -> Model {
        if bases.contains("Enum") {
            return Model::Alias(Self::enum_shape(body));
        }
        Model::Record(Self::record(bases, body))
    }

    fn record(bases: &str, body: &[&str]) -> Record {
        let total = !bases.contains("total=False");
        let mut record = Record {
            bases: Self::bases(bases),
            ..Record::default()
        };
        record.add_each(
            "py",
            body.iter().filter_map(|line| Self::field(line, total)),
        );
        record
    }

    fn field(line: &str, total: bool) -> Option<(String, String, bool)> {
        let (name, rest) = line.split_once(':')?;
        let name = Some(name.trim()).filter(|name| Self::is_field_name(name))?;
        let (kind, default) = rest
            .split_once('=')
            .map_or((rest.trim(), false), |(kind, _)| (kind.trim(), true));
        let not_required_kind = kind
            .strip_prefix("NotRequired[")
            .and_then(|inner| inner.strip_suffix(']'));
        let (kind, not_required) = not_required_kind.map_or((kind, false), |inner| (inner, true));
        let required = [total, !default, !not_required] == [true, true, true];
        Some((name.to_owned(), kind.to_owned(), required))
    }

    fn is_field_name(name: &str) -> bool {
        [
            name.bytes().all(Members::is_word_byte),
            !name.is_empty(),
            !name.starts_with("def"),
        ] == [true, true, true]
    }

    fn bases(text: &str) -> Vec<String> {
        TypeText::split_top_level(text, ',')
            .iter()
            .filter(|base| !base.contains('='))
            .filter(|base| !base.trim().is_empty())
            .map(|base| Members::base_name(base))
            .filter(|base| !Self::PLAIN_BASES.contains(&base.as_str()))
            .collect()
    }

    fn enum_shape(body: &[&str]) -> Shape {
        body.iter()
            .find_map(|line| Self::enum_value_shape(line))
            .unwrap_or(Shape::Any)
    }

    fn enum_value_shape(line: &str) -> Option<Shape> {
        let (_, value) = line.split_once('=')?;
        let value = value.trim();
        if value.starts_with(['"', '\'']) {
            Some(Shape::Text)
        } else if value.parse::<i64>().is_ok() {
            Some(Shape::Integer)
        } else {
            None
        }
    }
}

struct GoModels;

impl GoModels {
    fn read(code: &str) -> Vec<(String, Model)> {
        Lines::models(code, Self::type_declaration)
    }

    fn type_declaration(code: &str, at: usize, line: &str) -> Option<(String, Model)> {
        let rest = line.strip_prefix("type ")?;
        let name = rest.split_whitespace().next()?;
        let name = name.split('[').next().unwrap_or(name);
        let kind = Self::after_type_parameters(&rest[name.len()..]);
        if kind.starts_with("struct") {
            let (start, end) = Members::body_after("go", code, at)?;
            return Some((
                name.to_owned(),
                Model::Record(Self::record(&code[start..end])),
            ));
        }
        let kind = kind.trim_start_matches('=').trim();
        let shape = TypeText::shape("go", kind).unwrap_or(Shape::Any);
        Some((name.to_owned(), Model::Alias(shape)))
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
        let declarations = body
            .lines()
            .map(|line| Self::tagged(line.trim()))
            .filter(|(declaration, _)| !declaration.is_empty());
        for (declaration, tag) in declarations {
            Self::add_declaration(&mut record, declaration, tag);
        }
        record
    }

    fn tagged(line: &str) -> (&str, &str) {
        line.find('`')
            .map_or((line, ""), |at| (line[..at].trim(), &line[at..]))
    }

    fn add_declaration(record: &mut Record, declaration: &str, tag: &str) {
        if declaration.contains([' ', '\t', ',']) {
            Self::add_fields(record, declaration, tag);
        } else {
            Self::embedded(record, declaration, tag);
        }
    }

    fn add_fields(record: &mut Record, declaration: &str, tag: &str) {
        let (names, kind) = Self::names_and_kind(declaration);
        for name in names.into_iter().filter(|name| Self::is_exported(name)) {
            Self::add_field(record, name, kind, tag);
        }
    }

    fn is_exported(name: &str) -> bool {
        name.starts_with(|character: char| character.is_ascii_uppercase())
    }

    fn add_field(record: &mut Record, name: &str, kind: &str, tag: &str) {
        let (json_name, omit_empty) = Self::json_name(name, tag);
        if json_name != "-" {
            record.add("go", &json_name, kind, !omit_empty);
        }
    }

    fn embedded(record: &mut Record, kind: &str, tag: &str) {
        let written = Members::base_name(kind);
        let (json_name, omit_empty) = Self::json_name(&written, tag);
        match json_name.as_str() {
            "-" => {}
            json_name if json_name == written => record.bases.push(written),
            json_name => record.add("go", json_name, kind, !omit_empty),
        }
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
        let value = Self::json_tag(tag).unwrap_or("");
        let mut parts = value.split(',');
        let written = parts.next().unwrap_or("");
        let omit_empty = parts.any(|part| part == "omitempty");
        let json_name = Some(written)
            .filter(|written| !written.is_empty())
            .unwrap_or(name);
        (json_name.to_owned(), omit_empty)
    }

    fn json_tag(tag: &str) -> Option<&str> {
        let start = tag.find("json:\"")?;
        tag[start + 6..].split('"').next()
    }
}

struct Declaration<'a> {
    kind: &'static str,
    name: &'a str,
    offset: usize,
}

struct BracedModels;

impl BracedModels {
    const KEYWORDS: [&'static str; 4] = ["class", "record", "struct", "enum"];
    const RECORD_MODIFIERS: [&'static str; 2] = ["struct", "class"];
    const WORD_SEPARATORS: [char; 5] = ['(', '{', '<', ':', ';'];
    const MEMBER_ENDS: &'static [u8] = b";\n}";
    const SET_VISIBILITY: [&'static str; 3] = ["public(set)", "protected(set)", "private(set)"];
    const PHP_MODIFIERS: [&'static str; 5] =
        ["public", "protected", "private", "readonly", "final"];
    const HIDDEN_MODIFIERS: [&'static str; 2] = ["private", "protected"];
    const SKIPPED_WORDS: [&'static str; 3] = [" static ", "function ", "fun "];
    const MODIFIERS: [&'static str; 16] = [
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
    ];

    fn read(extension: &str, code: &str) -> Vec<(String, Model)> {
        Lines::starts(code)
            .into_iter()
            .filter_map(|(at, line)| {
                let declaration = Self::declaration(line)?;
                Some(Self::declared_model(
                    extension,
                    code,
                    at,
                    line,
                    &declaration,
                ))
            })
            .collect()
    }

    fn declared_model(
        extension: &str,
        code: &str,
        at: usize,
        line: &str,
        declaration: &Declaration<'_>,
    ) -> (String, Model) {
        let model = match declaration.kind {
            "enum" => Model::Alias(Self::enum_shape(extension, line)),
            _ => Model::Record(Self::record(extension, code, at, declaration)),
        };
        (declaration.name.to_owned(), model)
    }

    fn declaration(line: &str) -> Option<Declaration<'_>> {
        let words = Self::words(line);
        let (index, kind) = words
            .iter()
            .enumerate()
            .find_map(|(index, (_, word))| Self::keyword(word).map(|kind| (index, kind)))?;
        let name = Self::record_name(&words, index, kind)?;
        Self::named_declaration(&words, index, kind, name)
    }

    fn words(line: &str) -> Vec<(usize, &str)> {
        let mut offset = 0;
        line.split_inclusive(Self::is_word_separator)
            .map(|piece| {
                let start = offset;
                offset += piece.len();
                let word = piece.strip_suffix(Self::is_word_separator).unwrap_or(piece);
                (start, word)
            })
            .filter(|(_, word)| !word.is_empty())
            .collect()
    }

    fn is_word_separator(character: char) -> bool {
        [
            character.is_whitespace(),
            Self::WORD_SEPARATORS.contains(&character),
        ]
        .contains(&true)
    }

    fn keyword(word: &str) -> Option<&'static str> {
        Self::KEYWORDS.into_iter().find(|keyword| *keyword == word)
    }

    fn record_name<'a>(words: &[(usize, &'a str)], index: usize, kind: &str) -> Option<&'a str> {
        let name = words.get(index + 1)?.1;
        if kind == "record" && Self::RECORD_MODIFIERS.contains(&name) {
            return Some(words.get(index + 2)?.1);
        }
        Some(name)
    }

    fn named_declaration<'a>(
        words: &[(usize, &'a str)],
        index: usize,
        kind: &'static str,
        name: &'a str,
    ) -> Option<Declaration<'a>> {
        let offset = words[index].0;
        if name == "class" {
            let name = words.get(index + 2)?.1;
            return Some(Declaration {
                kind: "enum",
                name,
                offset,
            });
        }
        Some(Declaration { kind, name, offset })
    }

    fn enum_shape(extension: &str, line: &str) -> Shape {
        let integer_backed = [extension == "php", line.contains(": int")] == [true, true];
        let integer = integer_backed.then_some(Shape::Integer);
        integer.unwrap_or(Shape::Text)
    }

    fn record(extension: &str, code: &str, at: usize, declaration: &Declaration<'_>) -> Record {
        let name_end = Self::name_end(code, at, declaration);
        let primary = Self::primary_parameters(extension, code, name_end);
        let after_header = primary.map_or(name_end, |(_, close)| close + 1);
        let header_end = after_header
            + code[after_header..]
                .find(['{', ';'])
                .unwrap_or(code.len() - after_header);
        let tail = &code[after_header..header_end];
        let first_line = tail.trim_start().lines().next().unwrap_or("");
        let mut record = Record {
            bases: Self::bases(extension, first_line),
            ..Record::default()
        };
        let parameters = primary.map_or("", |(open, close)| &code[open..close]);
        Self::parameters(extension, parameters, &mut record);
        let body = Self::member_body(extension, code, header_end, tail);
        Self::members(extension, body.unwrap_or_default(), &mut record);
        record
    }

    fn name_end(code: &str, at: usize, declaration: &Declaration<'_>) -> usize {
        let keyword_end = at + declaration.offset + declaration.kind.len();
        keyword_end
            + code[keyword_end..].find(declaration.name).unwrap_or(0)
            + declaration.name.len()
    }

    fn primary_parameters(extension: &str, code: &str, name_end: usize) -> Option<(usize, usize)> {
        let next_stop = code[name_end..]
            .find(['{', ';', '('])
            .map(|stop| code.as_bytes()[name_end + stop]);
        (next_stop == Some(b'('))
            .then(|| Members::parenthesized_after(extension, code, name_end))
            .flatten()
    }

    fn member_body<'a>(
        extension: &str,
        code: &'a str,
        header_end: usize,
        tail: &str,
    ) -> Option<&'a str> {
        let another_type = tail
            .lines()
            .any(|line| Self::declaration(line.trim()).is_some());
        let opens_body = [
            code.as_bytes().get(header_end) == Some(&b'{'),
            !another_type,
        ] == [true, true];
        let body = opens_body
            .then(|| Members::body_after(extension, code, header_end))
            .flatten();
        body.map(|(open, close)| &code[open..close])
    }

    fn bases(extension: &str, header: &str) -> Vec<String> {
        let header = header.split(" where ").next().unwrap_or("");
        let list = Self::base_list(extension, header);
        let mut bases: Vec<String> = TypeText::split_top_level(list, ',')
            .iter()
            .filter(|base| !base.is_empty())
            .filter(|base| Self::keeps_kotlin_base(extension, base))
            .map(|base| Members::base_name(base))
            .filter(|base| Self::keeps_csharp_base(extension, base))
            .collect();
        bases.extend(Self::dart_mixins(extension, header));
        bases
    }

    fn base_list<'a>(extension: &str, header: &'a str) -> &'a str {
        match extension {
            "kt" | "cs" => header.trim_start().strip_prefix(':').unwrap_or(""),
            _ => Self::after_word(header, "extends"),
        }
    }

    fn keeps_kotlin_base(extension: &str, base: &str) -> bool {
        [extension != "kt", base.contains('(')].contains(&true)
    }

    fn keeps_csharp_base(extension: &str, base: &str) -> bool {
        [extension != "cs", !Self::csharp_interface(base)].contains(&true)
    }

    fn dart_mixins(extension: &str, header: &str) -> Vec<String> {
        let mixins = (extension == "dart").then(|| Self::after_word(header, "with"));
        TypeText::split_top_level(mixins.unwrap_or(""), ',')
            .iter()
            .filter(|base| !base.is_empty())
            .map(|base| Members::base_name(base))
            .collect()
    }

    fn after_word<'a>(header: &'a str, word: &str) -> &'a str {
        let marker = format!("{word} ");
        let at = header
            .match_indices(&marker)
            .map(|(at, _)| at)
            .find(|&at| Self::starts_word(header, at));
        at.map_or("", |at| {
            Self::until_next_clause(&header[at + marker.len()..])
        })
    }

    fn starts_word(header: &str, at: usize) -> bool {
        at.checked_sub(1)
            .is_none_or(|before| header.as_bytes()[before].is_ascii_whitespace())
    }

    fn until_next_clause(rest: &str) -> &str {
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

    fn without_default(text: &str) -> (&str, bool) {
        text.split_once('=')
            .map_or((text, false), |(declaration, _)| (declaration.trim(), true))
    }

    fn parameters(extension: &str, list: &str, record: &mut Record) {
        let parameters = Members::split(extension, list);
        let fields = parameters
            .iter()
            .filter_map(|parameter| Self::parameter(extension, parameter));
        record.add_each(extension, fields);
    }

    fn parameter(extension: &str, parameter: &str) -> Option<(String, String, bool)> {
        let (attributes, parameter) = Members::attributes(parameter);
        let (declaration, default) = Self::without_default(parameter);
        let (name, kind) = Self::name_and_type(extension, declaration)?;
        let name = Self::written_name(&attributes).unwrap_or(name);
        Some((name, kind, !default))
    }

    fn members(extension: &str, body: &str, record: &mut Record) {
        let bytes = body.as_bytes();
        let mut pending = Vec::new();
        let mut constructor = None;
        let mut start = 0;
        let member_ends =
            Members::top_level_positions(extension, bytes, 0, Self::brace_depth_change)
                .filter(|&at| Self::MEMBER_ENDS.contains(&bytes[at]));
        for at in member_ends {
            let end = Self::member_end(bytes, at);
            let text = body[start..end].trim();
            Self::add_member(extension, text, record, &mut pending, &mut constructor);
            start = at + 1;
        }
        let text = body[start..].trim();
        Self::add_member(extension, text, record, &mut pending, &mut constructor);
        Self::require_promoted(record, constructor);
    }

    fn brace_depth_change(bytes: &[u8], at: usize) -> i32 {
        match bytes[at] {
            b'{' | b'(' => 1,
            b'}' | b')' => -1,
            _ => 0,
        }
    }

    fn member_end(bytes: &[u8], at: usize) -> usize {
        if bytes[at] == b'}' { at + 1 } else { at }
    }

    fn require_promoted(record: &mut Record, constructor: Option<Vec<String>>) {
        let Some(promoted) = constructor else {
            return;
        };
        record
            .fields
            .iter_mut()
            .filter(|field| !promoted.contains(&field.name))
            .for_each(|field| field.required = false);
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
            Self::add_constructor(&text[at..], record, pending, constructor);
        } else if extension == "php" && text.starts_with("use ") {
            Self::add_traits(record, text);
        } else {
            Self::add_property(extension, text, record, pending);
        }
    }

    fn add_constructor(
        text: &str,
        record: &mut Record,
        pending: &mut Vec<String>,
        constructor: &mut Option<Vec<String>>,
    ) {
        let parameters = Members::parenthesized_after("php", text, 0);
        let promoted =
            parameters.map(|(start, end)| PhpParameters::read(&text[start..end], record));
        *constructor = promoted.or(constructor.take());
        pending.clear();
    }

    fn add_traits(record: &mut Record, text: &str) {
        record
            .bases
            .extend(text[4..].split(',').map(Members::base_name));
    }

    fn add_property(extension: &str, text: &str, record: &mut Record, pending: &mut Vec<String>) {
        let (attributes, rest) = Members::attributes(text);
        pending.extend(attributes.iter().map(|attribute| (*attribute).to_owned()));
        let declares = !rest.trim().is_empty();
        let written = declares.then(|| std::mem::take(pending));
        let member = written.and_then(|written| Self::written_member(extension, rest, &written));
        record.add_each(extension, member);
    }

    fn written_member(
        extension: &str,
        text: &str,
        written: &[String],
    ) -> Option<(String, String, bool)> {
        let written: Vec<&str> = written.iter().map(String::as_str).collect();
        let (name, kind, required) = Self::member(extension, text)?;
        let name = Self::written_name(&written).unwrap_or(name);
        Some((name, kind, required))
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
        let written = text.trim().trim_end_matches(',');
        let hidden = [extension == "php", Self::has_hidden_modifier(written)] == [true, true];
        let text = &Self::without_set_visibility(written);
        if hidden || Self::is_skipped(text) {
            return None;
        }
        let declaration = text.find("{ get").map_or(text.as_str(), |at| &text[..at]);
        let (declaration, initialized) = declaration
            .split_once('=')
            .map_or((declaration.trim(), false), |(declaration, _)| {
                (declaration.trim(), true)
            });
        let initialized = [initialized, Self::has_property_initializer(text)].contains(&true);
        let (name, kind) = Self::name_and_type(extension, declaration)?;
        Some((name, kind, !initialized))
    }

    fn has_hidden_modifier(text: &str) -> bool {
        Self::php_modifiers(text)
            .iter()
            .any(|modifier| Self::HIDDEN_MODIFIERS.contains(&modifier.as_str()))
    }

    fn is_skipped(text: &str) -> bool {
        let is_method = [text.contains('('), !text.contains("{ get")] == [true, true];
        let has_skipped_word = Self::SKIPPED_WORDS.iter().any(|word| text.contains(word));
        [
            text.is_empty(),
            text.starts_with("static "),
            has_skipped_word,
            is_method,
        ]
        .contains(&true)
    }

    fn has_property_initializer(text: &str) -> bool {
        [text.contains("{ get"), text.contains("} =")] == [true, true]
    }

    fn without_set_visibility(text: &str) -> String {
        let mut text = text.to_owned();
        for written in Self::SET_VISIBILITY {
            text = text.replace(written, "");
        }
        text
    }

    fn php_modifiers(text: &str) -> Vec<String> {
        let mut modifiers = Vec::new();
        let mut rest = text.trim_start();
        while let Some((modifier, after)) = Self::php_modifier(rest) {
            modifiers.push(modifier);
            rest = after;
        }
        modifiers
    }

    fn php_modifier(rest: &str) -> Option<(String, &str)> {
        let word = Members::identifier(rest);
        let lower = Some(word.to_ascii_lowercase())
            .filter(|lower| Self::PHP_MODIFIERS.contains(&lower.as_str()))?;
        let rest = &rest[word.len()..];
        let (modifier, rest) = rest
            .strip_prefix("(set)")
            .map_or((lower, rest), |after| (String::from("set"), after));
        Some((modifier, rest.trim_start()))
    }

    fn name_and_type(extension: &str, declaration: &str) -> Option<(String, String)> {
        let text = Self::without_modifiers(extension, declaration);
        Self::split_name_and_type(extension, &text)
    }

    fn without_modifiers(extension: &str, declaration: &str) -> String {
        let mut text = declaration.trim().to_owned();
        while let Some(rest) = Self::without_modifier(extension, &text) {
            text = rest;
        }
        text
    }

    fn without_modifier(extension: &str, text: &str) -> Option<String> {
        Self::MODIFIERS
            .iter()
            .find(|modifier| Self::starts_with_modifier(extension, text, modifier))
            .map(|modifier| text[modifier.len()..].trim_start().to_owned())
    }

    fn starts_with_modifier(extension: &str, text: &str, modifier: &str) -> bool {
        text.get(..modifier.len())
            .is_some_and(|start| Self::is_modifier(extension, start, modifier))
    }

    fn is_modifier(extension: &str, start: &str, modifier: &str) -> bool {
        match extension {
            "php" => start.eq_ignore_ascii_case(modifier),
            _ => start == modifier,
        }
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
        let parts = Members::split("php", list);
        for parameter in parts.iter().filter_map(|part| Self::parameter(part)) {
            Self::add_parameter(record, &mut promoted, parameter);
        }
        promoted
    }

    fn parameter(text: &str) -> Option<(String, String, bool, bool)> {
        let (_, parameter) = Members::attributes(text);
        let modifiers = BracedModels::php_modifiers(parameter);
        let becomes_property =
            [!modifiers.is_empty(), Self::is_visible(&modifiers)] == [true, true];
        let (declaration, default) = BracedModels::without_default(parameter);
        let declaration = BracedModels::without_set_visibility(declaration);
        let (name, kind) = BracedModels::name_and_type("php", &declaration)?;
        Some((name, kind, default, becomes_property))
    }

    fn is_visible(modifiers: &[String]) -> bool {
        modifiers.iter().any(|modifier| modifier == "public")
            || !modifiers
                .iter()
                .any(|modifier| BracedModels::HIDDEN_MODIFIERS.contains(&modifier.as_str()))
    }

    fn add_parameter(
        record: &mut Record,
        promoted: &mut Vec<String>,
        (name, kind, default, becomes_property): (String, String, bool, bool),
    ) {
        if becomes_property {
            record.add("php", &name, &kind, !default);
            promoted.push(name);
        } else if !default && record.unreadable.is_none() {
            record.unreadable = Some(format!(
                "the constructor needs ${name}, but ${name} is not a public property, so a request body cannot fill it. Make it public (public {kind} ${name}) or give it a default value"
            ));
        }
    }
}
