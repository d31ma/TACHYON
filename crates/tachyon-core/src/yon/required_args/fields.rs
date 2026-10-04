use super::{Failure, Input, identifier};
use crate::annotation_rules::AnnotationRules;
use crate::python_parameters::PythonAnnotated;
use std::ops::Range;

pub(super) struct Field {
    pub name: String,
    pub ty: String,
    pub eligible: bool,
    pub autowired: bool,
    pub qualifier: Option<String>,
}

impl Field {
    const MAX_DIRECT_FIELDS: usize = 256;
}

#[derive(Default)]
struct BracedFields {
    start: usize,
    depth: usize,
    parentheses: usize,
    brackets: usize,
    angles: usize,
    method: bool,
}

impl BracedFields {
    fn read(&mut self, code: &str, offset: usize, byte: u8) {
        match byte {
            b'{' => self.open_brace(code, offset),
            b'}' => {
                self.depth = self.depth.saturating_sub(1);
                if self.depth == 0 && self.method {
                    self.start = offset + 1;
                    self.method = false;
                }
            }
            b'(' if self.depth == 0 => self.parentheses += 1,
            b')' if self.depth == 0 => self.parentheses = self.parentheses.saturating_sub(1),
            b'[' if self.depth == 0 => self.brackets += 1,
            b']' if self.depth == 0 => self.brackets = self.brackets.saturating_sub(1),
            b'<' if self.depth == 0 => self.angles += 1,
            b'>' if self.depth == 0 => self.angles = self.angles.saturating_sub(1),
            _ => {}
        }
    }

    fn open_brace(&mut self, code: &str, offset: usize) {
        if self.depth == 0 {
            let prefix = code[self.start..offset].trim_start();
            self.method = !prefix.contains('=')
                && !prefix.starts_with(['@', '#', '['])
                && self.parentheses == 0;
        }
        self.depth += 1;
    }

    fn ends_field(&self, byte: u8, extension: &str) -> bool {
        matches!(byte, b';' | b'\n' | b',')
            && self.depth == 0
            && self.parentheses == 0
            && self.brackets == 0
            && (extension != "rs" || self.angles == 0)
            && (byte != b',' || extension == "rs")
    }
}

#[derive(Default)]
pub(super) struct Pending {
    pub autowired: bool,
    pub qualifier: Option<String>,
    pub spans: Vec<Range<usize>>,
}

pub(super) fn collect(input: &Input<'_>, body: &Range<usize>) -> Result<Vec<Field>, Failure> {
    collect_with_markers(input, body).map(|(fields, _)| fields)
}

pub(super) fn collect_with_markers(
    input: &Input<'_>,
    body: &Range<usize>,
) -> Result<(Vec<Field>, Vec<Range<usize>>), Failure> {
    let mut pending = Pending::default();
    let fields = if input.extension == "py" {
        python(input, body, &mut pending)?
    } else {
        braced(input, body, &mut pending)?
    };
    Ok((fields, pending.spans))
}

fn braced(
    input: &Input<'_>,
    body: &Range<usize>,
    pending: &mut Pending,
) -> Result<Vec<Field>, Failure> {
    let mut output = Vec::new();
    let code = &input.code[body.clone()];
    let mut scanner = BracedFields::default();
    for (offset, byte) in code.bytes().enumerate() {
        scanner.read(code, offset, byte);
        if !scanner.ends_field(byte, input.extension) {
            continue;
        }
        append(
            input,
            body.start + scanner.start,
            &code[scanner.start..offset],
            &mut output,
            pending,
        )?;
        scanner.start = offset + 1;
    }
    if scanner.start < code.len() {
        append(
            input,
            body.start + scanner.start,
            &code[scanner.start..],
            &mut output,
            pending,
        )?;
    }
    Ok(output)
}

fn python(
    input: &Input<'_>,
    body: &Range<usize>,
    pending: &mut Pending,
) -> Result<Vec<Field>, Failure> {
    let code = &input.code[body.clone()];
    let authored = &input.source[body.clone()];
    let every = PythonAnnotated::markers(code, authored, |written| {
        AnnotationRules::find(written).is_some_and(crate::annotation_rules::Rule::python_annotated)
    });
    let injection = PythonAnnotated::markers(code, authored, |written| {
        matches!(normalized(written).as_str(), "needs" | "named")
    });
    pending.spans.extend(
        injection
            .erased
            .iter()
            .map(|range| body.start + range.start..body.start + range.end),
    );
    let plain = every.blanked(code);
    let indent = plain
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start().len())
        .min()
        .unwrap_or(0);
    let mut output = Vec::new();
    let mut offset = 0;
    for line in plain.split_inclusive('\n') {
        if line.len() - line.trim_start().len() == indent {
            for marker in &injection.markers {
                if (offset..offset + line.len()).contains(&marker.declaration) {
                    record(
                        input,
                        body.start + marker.at,
                        &normalized(&marker.written),
                        &marker.arguments,
                        pending,
                    )?;
                }
            }
            append(input, body.start + offset, line, &mut output, pending)?;
        }
        offset += line.len();
    }
    Ok(output)
}

fn normalized(written: &str) -> String {
    written.replace('_', "").to_ascii_lowercase()
}

fn append(
    input: &Input<'_>,
    offset: usize,
    code: &str,
    output: &mut Vec<Field>,
    pending: &mut Pending,
) -> Result<(), Failure> {
    let raw = code;
    let code = code.trim();
    if code.is_empty() || matches!(code, "pass" | "...") {
        return Ok(());
    }
    if code.starts_with(['@', '#', '[']) {
        return marker(input, offset, raw, code, pending);
    }
    let Some(mut field) = parse_field(input, offset, code)? else {
        return Ok(());
    };
    field.autowired = std::mem::take(&mut pending.autowired);
    field.qualifier = pending.qualifier.take();
    output.push(field);
    if output.len() > Field::MAX_DIRECT_FIELDS {
        return Err(input.fail(
            offset,
            "Managed constructor types permit at most 256 direct fields",
        ));
    }
    Ok(())
}

fn parse_field(input: &Input<'_>, offset: usize, code: &str) -> Result<Option<Field>, Failure> {
    let assignment = assignment(code);
    let initialized = assignment.is_some() || code.split_whitespace().any(|word| word == "by");
    let declaration = code[..assignment.unwrap_or(code.len())].trim();
    if input.extension == "go" && declaration.split_whitespace().count() < 2 {
        return Ok(None);
    }
    if skip_member(input, offset, declaration, initialized)? {
        return Ok(None);
    }
    let is_static = declaration
        .split_whitespace()
        .any(|word| matches!(word, "static" | "const" | "class"));
    let type_only =
        input.extension == "ts" && declaration.split_whitespace().any(|word| word == "declare");
    let Some((written_name, ty)) = declaration_parts(input.extension, declaration) else {
        return Err(input.fail(offset, "Cannot infer dependencies from this member; use simple direct instance field declarations"));
    };
    let optional = input.extension == "ts" && written_name.ends_with('?');
    let name = if optional {
        written_name.trim_end_matches('?').to_owned()
    } else {
        written_name
    };
    if name.starts_with(['$', '#']) {
        return Ok(None);
    }
    if !identifier(&name) {
        return Err(input.fail(
            offset,
            "Required constructor fields must be simple identifiers",
        ));
    }
    let class_variable = input.extension == "py"
        && (ty.starts_with("ClassVar[") || ty.starts_with("typing.ClassVar["));
    Ok(Some(Field {
        name,
        ty,
        eligible: !initialized && !optional && !is_static && !class_variable && !type_only,
        autowired: false,
        qualifier: None,
    }))
}

fn skip_member(
    input: &Input<'_>,
    offset: usize,
    declaration: &str,
    initialized: bool,
) -> Result<bool, Failure> {
    if let Some(opening) = declaration.find('(') {
        let field_type = declaration[..opening].contains(':')
            || input.extension == "go"
            || (input.extension == "rs"
                && declaration.starts_with("pub(")
                && declaration.contains(':'));
        if field_type && !initialized {
            return Err(input.fail(offset, "Required fields must use concrete provider types, not function or parenthesized types"));
        }
        if !field_type {
            return Ok(true);
        }
    }
    Ok(declaration.contains('{') || declaration.starts_with("def "))
}

fn assignment(code: &str) -> Option<usize> {
    let bytes = code.as_bytes();
    for (offset, byte) in bytes.iter().enumerate() {
        if *byte != b'=' || matches!(bytes.get(offset + 1), Some(b'>' | b'=')) {
            continue;
        }
        let previous = offset.checked_sub(1).and_then(|offset| bytes.get(offset));
        if previous != Some(&b'=') {
            return Some(offset);
        }
    }
    None
}

fn declaration_parts(extension: &str, declaration: &str) -> Option<(String, String)> {
    match extension {
        "js" => javascript_field(declaration),
        "ts" | "py" | "kt" | "rs" => colon_field(declaration),
        "java" | "cs" | "php" | "dart" => type_first_field(declaration),
        "go" => name_first_field(declaration),
        _ => None,
    }
}

fn javascript_field(declaration: &str) -> Option<(String, String)> {
    let words: Vec<_> = declaration.split_whitespace().collect();
    match words.as_slice() {
        [name] | ["static", name] => Some(((*name).to_owned(), String::new())),
        _ => None,
    }
}

fn colon_field(declaration: &str) -> Option<(String, String)> {
    let (prefix, ty) = declaration.split_once(':').unwrap_or((declaration, ""));
    let words: Vec<_> = prefix.split_whitespace().collect();
    let name = words.last()?.trim_end_matches('!');
    for word in &words[..words.len() - 1] {
        if !matches!(
            *word,
            "public"
                | "private"
                | "protected"
                | "internal"
                | "readonly"
                | "declare"
                | "abstract"
                | "static"
                | "pub"
                | "val"
                | "var"
                | "lateinit"
                | "final"
        ) {
            return None;
        }
    }
    Some((name.to_owned(), ty.trim().to_owned()))
}

fn name_first_field(declaration: &str) -> Option<(String, String)> {
    let (name, ty) = declaration.split_once(char::is_whitespace)?;
    Some((name.to_owned(), ty.trim().to_owned()))
}

fn type_first_field(declaration: &str) -> Option<(String, String)> {
    let words: Vec<_> = declaration
        .split_whitespace()
        .filter(|word| !type_first_modifier(word))
        .collect();
    if words.len() != 2 {
        return None;
    }
    Some((
        words[1].trim_start_matches('$').to_owned(),
        words[0].to_owned(),
    ))
}

fn type_first_modifier(word: &str) -> bool {
    matches!(
        word,
        "public"
            | "private"
            | "protected"
            | "internal"
            | "readonly"
            | "final"
            | "static"
            | "const"
            | "volatile"
            | "transient"
            | "late"
            | "covariant"
    )
}

fn marker(
    input: &Input<'_>,
    offset: usize,
    raw: &str,
    code: &str,
    pending: &mut Pending,
) -> Result<(), Failure> {
    let start = offset + (raw.len() - raw.trim_start().len());
    let span = start..start + code.trim_end().len();
    let authored = input.source.get(span.clone()).unwrap_or(code);
    let trimmed = authored.trim_end().trim_end_matches(']');
    let rest = trimmed
        .trim_start_matches(['@', '#', '['])
        .trim_start_matches("yon::")
        .trim_start_matches("tac::");
    let (name, arguments) = rest.split_once('(').map_or((rest, ""), |(name, rest)| {
        (name, rest.trim_end_matches(')'))
    });
    let name = normalized(name.trim());
    if matches!(name.as_str(), "needs" | "named") {
        record(input, start, &name, arguments, pending)?;
        pending.spans.push(span);
    }
    Ok(())
}

fn record(
    input: &Input<'_>,
    start: usize,
    name: &str,
    arguments: &str,
    pending: &mut Pending,
) -> Result<(), Failure> {
    match name {
        "needs" => {
            if !arguments.trim().is_empty() {
                return Err(input.fail(start, AnnotationRules::no_arguments("Needs")));
            }
            if std::mem::replace(&mut pending.autowired, true) {
                return Err(input.fail(start, "One Needs per field"));
            }
        }
        "named" => {
            let value = arguments.trim().trim_matches(['"', '\'']).trim();
            if !super::identifier(value) {
                return Err(input.fail(start, "Named names one provider type"));
            }
            if pending.qualifier.replace(value.to_owned()).is_some() {
                return Err(input.fail(start, "One Named per field"));
            }
        }
        _ => {}
    }
    Ok(())
}
