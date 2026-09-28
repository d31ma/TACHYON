use super::super::{Model, Record};
use crate::ruby_source::Yard;
use std::ops::Range;

pub(super) struct RubyModels;

struct RubyDeclaration {
    name: String,
    base: Option<String>,
    members: Vec<String>,
    region: Range<usize>,
}

impl RubyModels {
    const RECORD_BUILDERS: [&'static str; 2] = ["Data.define", "Struct.new"];
    const ATTRIBUTE_METHODS: [&'static str; 2] = ["attr_reader", "attr_accessor"];

    pub(super) fn read(source: &str) -> Vec<(String, Model)> {
        let code = Self::code(source);
        let mut models = Vec::new();
        let mut offset = 0;
        for line in code.split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            if line.starts_with([' ', '\t']) {
                continue;
            }
            let Some(declaration) = Self::declaration(&code, line.trim_end(), start) else {
                continue;
            };
            let record = Self::record(source, &code, &declaration);
            models.push((declaration.name, Model::Record(record)));
        }
        models
    }

    fn code(source: &str) -> String {
        let mask = crate::lexical::code_mask("rb", source);
        let mut bytes = source.as_bytes().to_vec();
        for (byte, visible) in bytes.iter_mut().zip(mask.code) {
            if !visible && !matches!(*byte, b'\n' | b'\r') {
                *byte = b' ';
            }
        }
        String::from_utf8(bytes).unwrap_or_default()
    }

    fn declaration(code: &str, line: &str, start: usize) -> Option<RubyDeclaration> {
        if let Some(rest) = line.strip_prefix("class ") {
            let name = Self::constant(rest)?;
            let after_name = rest[name.len()..].trim_start();
            let parent = after_name.strip_prefix('<').map(str::trim);
            let members = Self::members(code, start);
            let base = parent.filter(|parent| !Self::is_record_builder(parent));
            return Some(RubyDeclaration {
                name: Self::last_segment(name),
                base: base.and_then(Self::constant).map(Self::last_segment),
                members,
                region: start..Self::class_end(code, start),
            });
        }
        let name = Self::constant(line)?;
        let value = line[name.len()..]
            .trim_start()
            .strip_prefix('=')?
            .trim_start();
        if !Self::is_record_builder(value) {
            return None;
        }
        Some(RubyDeclaration {
            name: Self::last_segment(name),
            base: None,
            members: Self::members(code, start),
            region: start..Self::statement_end(code, start),
        })
    }

    fn record(source: &str, code: &str, declaration: &RubyDeclaration) -> Record {
        let mut comments = Yard::comments_before(source, declaration.region.start);
        comments.push('\n');
        comments.push_str(&Self::inner_comments(source, &declaration.region));
        let documented = Yard::attribute_types(&comments);
        let mut names = declaration.members.clone();
        let mut attribute_types = Vec::new();
        for (name, type_text) in Self::attributes(source, code, &declaration.region) {
            if !names.contains(&name) {
                names.push(name.clone());
            }
            attribute_types.push((name, type_text));
        }
        let mut record = Record {
            bases: declaration.base.iter().cloned().collect(),
            ..Record::default()
        };
        for name in names {
            let type_text = Self::type_of(&name, &documented, &attribute_types);
            record.add("rb", &name, &type_text, true);
        }
        record
    }

    fn type_of(
        name: &str,
        documented: &[(String, String)],
        attributes: &[(String, String)],
    ) -> String {
        for (attribute, type_text) in attributes.iter().chain(documented) {
            if attribute == name && !type_text.is_empty() {
                return type_text.clone();
            }
        }
        String::new()
    }

    fn attributes(source: &str, code: &str, region: &Range<usize>) -> Vec<(String, String)> {
        let mut found = Vec::new();
        let mut offset = region.start;
        for line in code[region.clone()].split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            let text = line.trim();
            let Some(rest) = Self::ATTRIBUTE_METHODS
                .iter()
                .find_map(|method| text.strip_prefix(method))
            else {
                continue;
            };
            let indent = line.len() - line.trim_start().len();
            let comments = Yard::comments_before(source, start + indent);
            let type_text = Yard::return_type(&comments);
            for name in Self::symbols(rest) {
                found.push((name, type_text.clone()));
            }
        }
        found
    }

    fn inner_comments(source: &str, region: &Range<usize>) -> String {
        let mut comments = String::new();
        for line in source[region.clone()].lines() {
            let text = line.trim();
            let Some(comment) = text.strip_prefix('#') else {
                continue;
            };
            if comment.starts_with('[') {
                continue;
            }
            comments.push_str(comment);
            comments.push('\n');
        }
        comments
    }

    fn members(code: &str, start: usize) -> Vec<String> {
        let line_end = code[start..].find('\n').map_or(code.len(), |at| start + at);
        let line = &code[start..line_end];
        let Some(builder) = Self::RECORD_BUILDERS
            .iter()
            .find_map(|builder| line.find(builder).map(|at| at + builder.len()))
        else {
            return Vec::new();
        };
        let opening = start + builder;
        if code.as_bytes().get(opening) != Some(&b'(') {
            return Vec::new();
        }
        let Some(closing) = Self::closing_parenthesis(code, opening) else {
            return Vec::new();
        };
        Self::symbols(&code[opening + 1..closing])
    }

    fn symbols(text: &str) -> Vec<String> {
        let mut names = Vec::new();
        for part in text.split(',') {
            let Some(name) = part.trim().strip_prefix(':') else {
                continue;
            };
            let valid = !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_');
            if valid {
                names.push(name.to_owned());
            }
        }
        names
    }

    fn closing_parenthesis(code: &str, opening: usize) -> Option<usize> {
        let mut depth = 0_usize;
        for (at, byte) in code.bytes().enumerate().skip(opening) {
            match byte {
                b'(' => depth += 1,
                b')' if depth == 1 => return Some(at),
                b')' => depth -= 1,
                _ => {}
            }
        }
        None
    }

    fn class_end(code: &str, start: usize) -> usize {
        let mut offset = start;
        for line in code[start..].split_inclusive('\n') {
            offset += line.len();
            if offset - line.len() > start && line.trim_end() == "end" {
                return offset;
            }
        }
        code.len()
    }

    fn statement_end(code: &str, start: usize) -> usize {
        let line_end = code[start..].find('\n').map_or(code.len(), |at| start + at);
        if !code[start..line_end].trim_end().ends_with(" do") {
            return line_end;
        }
        Self::class_end(code, start)
    }

    fn is_record_builder(text: &str) -> bool {
        Self::RECORD_BUILDERS
            .iter()
            .any(|builder| text.starts_with(builder))
    }

    fn constant(text: &str) -> Option<&str> {
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b':'))
            .count();
        let name = &text[..length];
        let capitalized = name.as_bytes().first().is_some_and(u8::is_ascii_uppercase);
        capitalized.then_some(name)
    }

    fn last_segment(name: &str) -> String {
        name.rsplit("::").next().unwrap_or(name).to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::RubyModels;
    use crate::route_types::{Model, Shape};

    fn fields(source: &str, name: &str) -> Vec<(String, Shape, bool)> {
        let models = RubyModels::read(source);
        let Some((_, Model::Record(record))) = models.iter().find(|(found, _)| found == name)
        else {
            panic!("{name} missing from {models:?}");
        };
        record
            .fields
            .iter()
            .map(|field| (field.name.clone(), field.shape.clone(), field.required))
            .collect()
    }

    #[test]
    fn data_members_take_their_types_from_attribute_directives() {
        let source = "# @!attribute [r] id\n#   @return [String]\n# @!attribute [r] lines\n#   @return [Array<Line>]\n# @!attribute [r] note\n#   @return [String, nil]\nOrder = Data.define(:id, :lines, :note)\n";
        assert_eq!(
            fields(source, "Order"),
            [
                (String::from("id"), Shape::Text, true),
                (
                    String::from("lines"),
                    Shape::List(Box::new(Shape::Model(String::from("Line")))),
                    true
                ),
                (
                    String::from("note"),
                    Shape::Optional(Box::new(Shape::Text)),
                    false
                ),
            ]
        );
    }

    #[test]
    fn a_class_reads_documented_attribute_readers_and_its_parent() {
        let source = "class Admin < User\n  # @return [Integer]\n  attr_reader :level\n  # @return [Boolean]\n  attr_accessor :active\n\n  def initialize(level:, active:)\n    @level = level\n  end\nend\n";
        let models = RubyModels::read(source);
        let Some((_, Model::Record(record))) = models.first() else {
            panic!("{models:?}");
        };
        assert_eq!(record.bases, [String::from("User")]);
        assert_eq!(
            fields(source, "Admin"),
            [
                (String::from("level"), Shape::Integer, true),
                (String::from("active"), Shape::Boolean, true),
            ]
        );
    }

    #[test]
    fn a_data_subclass_keeps_its_members_and_undocumented_members_are_any() {
        let source = "class Point < Data.define(:x, :y)\n  # @!attribute [r] x\n  #   @return [Float]\nend\n";
        assert_eq!(
            fields(source, "Point"),
            [
                (String::from("x"), Shape::Number, true),
                (String::from("y"), Shape::Any, true),
            ]
        );
    }
}
