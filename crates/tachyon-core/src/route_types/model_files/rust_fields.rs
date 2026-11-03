use super::rust_field::RustField;
use super::{BracedModels, Lines, Members};
use crate::route_types::{Model, Record};

pub(super) struct RustFields<'a> {
    body: &'a str,
    offset: usize,
    cursor: usize,
}

impl<'a> RustFields<'a> {
    const LANGUAGE: &'static str = "rs";
    const VISIBILITY: &'static str = "pub";

    pub(super) fn read(body: &'a str, offset: usize) -> Vec<RustField> {
        let mut reader = Self {
            body,
            offset,
            cursor: 0,
        };
        let members = Members::split(Self::LANGUAGE, body);
        reader.fields(&members)
    }

    pub(super) fn read_models(code: &'a str) -> Vec<(String, Vec<RustField>)> {
        Lines::starts(code)
            .into_iter()
            .filter_map(|(at, line)| Self::model(code, at, line))
            .collect()
    }

    pub(super) fn models(code: &'a str, models: Vec<(String, Model)>) -> Vec<(String, Model)> {
        let fields = Self::read_models(code);
        models
            .into_iter()
            .map(|model| Self::with_fields(model, &fields))
            .collect()
    }

    fn fields(&mut self, members: &[String]) -> Vec<RustField> {
        members
            .iter()
            .filter_map(|member| self.field(member))
            .collect()
    }

    fn field(&mut self, member: &str) -> Option<RustField> {
        let relative = self.body[self.cursor..].find(member)?;
        let offset = self.offset + self.cursor + relative;
        self.cursor += relative + member.len();
        let (_, text) = Members::attributes(member);
        let visible = Self::without_visibility(text);
        let scope = Self::scope_length(visible);
        let declaration = visible[scope..].trim_start();
        let name = Self::field_name(declaration);
        let rest = declaration[name.len()..].trim_start();
        let kind = rest.strip_prefix(':')?.trim();
        let name_start = offset + member.len() - declaration.len();
        let type_start = offset + member.len() - kind.len();
        Self::named_type(name, kind, name_start, type_start)
    }

    fn field_name(declaration: &str) -> &str {
        let raw = declaration
            .strip_prefix(RustField::RAW_PREFIX)
            .map_or(0, |_| RustField::RAW_PREFIX.len());
        &declaration[..raw + Members::identifier(&declaration[raw..]).len()]
    }

    fn without_visibility(text: &str) -> &str {
        match Members::identifier(text) {
            Self::VISIBILITY => text[Self::VISIBILITY.len()..].trim_start(),
            _ => text,
        }
    }

    fn scope_length(text: &str) -> usize {
        text.strip_prefix('(')
            .and_then(|_| Members::group_length(text, b'(', b')'))
            .unwrap_or(0)
    }

    fn named_type(
        name: &str,
        kind: &str,
        name_start: usize,
        type_start: usize,
    ) -> Option<RustField> {
        if [name, kind].contains(&"") {
            return None;
        }
        Some(RustField {
            name: name.to_owned(),
            type_text: kind.to_owned(),
            name_range: name_start..name_start + name.len(),
            type_range: type_start..type_start + kind.len(),
        })
    }

    fn model(code: &'a str, at: usize, line: &str) -> Option<(String, Vec<RustField>)> {
        let declaration = BracedModels::declaration(line).filter(|value| value.kind == "struct")?;
        let keyword_end = at + declaration.offset + declaration.kind.len();
        let name_end =
            keyword_end + code[keyword_end..].find(declaration.name)? + declaration.name.len();
        let header = code[name_end..].trim_start();
        let generic_length = header
            .strip_prefix('<')
            .and_then(|_| Members::group_length(header, b'<', b'>'))
            .unwrap_or(0);
        let header = header[generic_length..].trim_start();
        let header_start = code.len() - header.len();
        let body_start = header
            .find(['{', ';', '('])
            .map(|relative| header_start + relative)
            .filter(|start| code.as_bytes().get(*start) == Some(&b'{'))?;
        let (open, close) = Members::body_after(Self::LANGUAGE, code, body_start)?;
        Some((
            declaration.name.to_owned(),
            Self::read(&code[open..close], open),
        ))
    }

    fn with_fields(model: (String, Model), fields: &[(String, Vec<RustField>)]) -> (String, Model) {
        let (name, model) = model;
        let record = fields.iter().find(|(written, _)| written == &name);
        let model = record.map_or(model, |(_, fields)| Model::Record(Self::record(fields)));
        (name, model)
    }

    fn record(fields: &[RustField]) -> Record {
        let mut record = Record::default();
        for field in fields {
            record.add(Self::LANGUAGE, field.key(), &field.type_text, true);
        }
        record
    }
}
