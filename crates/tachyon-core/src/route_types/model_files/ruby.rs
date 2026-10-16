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
    const LINE_BREAKS: &'static [u8] = b"\n\r";
    const CONSTANT_SYMBOLS: &'static [u8] = b"_:";

    pub(super) fn read(source: &str) -> Vec<(String, Model)> {
        let code = Self::code(source);
        Self::lines_from(&code, 0)
            .filter(|(_, line)| !line.starts_with([' ', '\t']))
            .filter_map(|(start, line)| Self::declaration(&code, line.trim_end(), start))
            .map(|declaration| Self::model(source, &code, declaration))
            .collect()
    }

    fn lines_from(text: &str, first: usize) -> impl Iterator<Item = (usize, &str)> {
        let mut offset = first;
        text.split_inclusive('\n').map(move |line| {
            let start = offset;
            offset += line.len();
            (start, line)
        })
    }

    fn model(source: &str, code: &str, declaration: RubyDeclaration) -> (String, Model) {
        let record = Self::record(source, code, &declaration);
        (declaration.name, Model::Record(record))
    }

    fn code(source: &str) -> String {
        let mask = crate::lexical::code_mask("rb", source);
        let mut bytes = source.as_bytes().to_vec();
        for (byte, visible) in bytes.iter_mut().zip(mask.code) {
            *byte = Self::masked(*byte, visible);
        }
        String::from_utf8(bytes).unwrap_or_default()
    }

    fn masked(byte: u8, visible: bool) -> u8 {
        let keeps = [visible, Self::LINE_BREAKS.contains(&byte)].contains(&true);
        if keeps { byte } else { b' ' }
    }

    fn declaration(code: &str, line: &str, start: usize) -> Option<RubyDeclaration> {
        line.strip_prefix("class ").map_or_else(
            || Self::builder_declaration(code, line, start),
            |rest| Self::class_declaration(code, rest, start),
        )
    }

    fn class_declaration(code: &str, rest: &str, start: usize) -> Option<RubyDeclaration> {
        let name = Self::constant(rest)?;
        let after_name = rest[name.len()..].trim_start();
        let parent = after_name.strip_prefix('<').map(str::trim);
        let members = Self::members(code, start);
        let base = parent.filter(|parent| !Self::is_record_builder(parent));
        Some(RubyDeclaration {
            name: Self::last_segment(name),
            base: base.and_then(Self::constant).map(Self::last_segment),
            members,
            region: start..Self::class_end(code, start),
        })
    }

    fn builder_declaration(code: &str, line: &str, start: usize) -> Option<RubyDeclaration> {
        let name = Self::constant(line)?;
        let value = line[name.len()..]
            .trim_start()
            .strip_prefix('=')?
            .trim_start();
        Self::is_record_builder(value).then(|| RubyDeclaration {
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
        let attribute_types = Self::attributes(source, code, &declaration.region);
        let names = Self::member_names(&declaration.members, &attribute_types);
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

    fn member_names(members: &[String], attributes: &[(String, String)]) -> Vec<String> {
        let mut names = members.to_vec();
        for (name, _) in attributes {
            Self::add_once(&mut names, name);
        }
        names
    }

    fn add_once(names: &mut Vec<String>, name: &String) {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }

    fn type_of(
        name: &str,
        documented: &[(String, String)],
        attributes: &[(String, String)],
    ) -> String {
        attributes
            .iter()
            .chain(documented)
            .filter(|(attribute, _)| attribute == name)
            .find(|(_, type_text)| !type_text.is_empty())
            .map_or_else(String::new, |(_, type_text)| type_text.clone())
    }

    fn attributes(source: &str, code: &str, region: &Range<usize>) -> Vec<(String, String)> {
        Self::lines_from(&code[region.clone()], region.start)
            .filter_map(|(start, line)| Self::attribute_line(source, start, line))
            .flatten()
            .collect()
    }

    fn attribute_line(source: &str, start: usize, line: &str) -> Option<Vec<(String, String)>> {
        let text = line.trim();
        let rest = Self::ATTRIBUTE_METHODS
            .iter()
            .find_map(|method| text.strip_prefix(method))?;
        let indent = line.len() - line.trim_start().len();
        let comments = Yard::comments_before(source, start + indent);
        let type_text = Yard::return_type(&comments);
        let names = Self::symbols(rest).into_iter();
        Some(names.map(|name| (name, type_text.clone())).collect())
    }

    fn inner_comments(source: &str, region: &Range<usize>) -> String {
        let mut comments = String::new();
        for comment in source[region.clone()]
            .lines()
            .filter_map(Self::inner_comment)
        {
            comments.push_str(comment);
            comments.push('\n');
        }
        comments
    }

    fn inner_comment(line: &str) -> Option<&str> {
        let comment = line.trim().strip_prefix('#')?;
        (!comment.starts_with('[')).then_some(comment)
    }

    fn members(code: &str, start: usize) -> Vec<String> {
        Self::builder_arguments(code, start).map_or_else(Vec::new, Self::symbols)
    }

    fn builder_arguments(code: &str, start: usize) -> Option<&str> {
        let line_end = code[start..].find('\n').map_or(code.len(), |at| start + at);
        let line = &code[start..line_end];
        let builder = Self::RECORD_BUILDERS
            .iter()
            .find_map(|builder| line.find(builder).map(|at| at + builder.len()))?;
        let opening = start + builder;
        code.as_bytes().get(opening).filter(|&&byte| byte == b'(')?;
        let closing = Self::closing_parenthesis(code, opening)?;
        Some(&code[opening + 1..closing])
    }

    fn symbols(text: &str) -> Vec<String> {
        text.split(',').filter_map(Self::symbol).collect()
    }

    fn symbol(part: &str) -> Option<String> {
        let name = part.trim().strip_prefix(':')?;
        Self::is_symbol_name(name).then(|| name.to_owned())
    }

    fn is_symbol_name(name: &str) -> bool {
        !name.is_empty() && name.bytes().all(Self::is_word_byte)
    }

    fn is_word_byte(byte: u8) -> bool {
        [byte.is_ascii_alphanumeric(), byte == b'_'].contains(&true)
    }

    fn closing_parenthesis(code: &str, opening: usize) -> Option<usize> {
        let mut depth = 0_usize;
        let closing = code
            .bytes()
            .enumerate()
            .skip(opening)
            .find(|&(_, byte)| Self::closes_group(byte, &mut depth));
        closing.map(|(at, _)| at)
    }

    fn closes_group(byte: u8, depth: &mut usize) -> bool {
        match byte {
            b'(' => *depth += 1,
            b')' if *depth == 1 => return true,
            b')' => *depth -= 1,
            _ => {}
        }
        false
    }

    fn class_end(code: &str, start: usize) -> usize {
        Self::lines_from(&code[start..], start)
            .filter(|&(line_start, _)| line_start > start)
            .find(|(_, line)| line.trim_end() == "end")
            .map_or(code.len(), |(line_start, line)| line_start + line.len())
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
            .take_while(|&byte| Self::is_constant_byte(byte))
            .count();
        let name = &text[..length];
        let capitalized = name.as_bytes().first().is_some_and(u8::is_ascii_uppercase);
        capitalized.then_some(name)
    }

    fn is_constant_byte(byte: u8) -> bool {
        [
            byte.is_ascii_alphanumeric(),
            Self::CONSTANT_SYMBOLS.contains(&byte),
        ]
        .contains(&true)
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
