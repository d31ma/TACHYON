use super::{Field, Input, Method, Parameter, Type, is_reserved_type_name};
use crate::Failure;
use crate::ruby_source::Yard;
use std::collections::BTreeSet;
use std::ops::Range;

pub(super) struct RubySyntax;

impl RubySyntax {
    const FIELD_WORDS: [&'static str; 3] = ["attr_reader ", "attr_accessor ", "attr_writer "];
    const PLAIN_PARAMETERS: &'static str = "Behavior methods require explicit positional parameters without defaults or rest arguments";
    const NEEDS_PARENTHESES: &'static str =
        "Put the parameters of an annotated Ruby method in parentheses, for example `def find(id)`";
    const UNTYPED: &'static str = "Object";

    pub(super) fn lowered_markers(
        language: &str,
        environment: &str,
        markers: Vec<super::Marker>,
    ) -> Vec<super::Marker> {
        if language != "rb" {
            return markers;
        }
        markers
            .into_iter()
            .filter(|marker| {
                let container_marker = matches!(marker.name.as_str(), "Needs" | "Named");
                let yon_needs_all = marker.name == "NeedsAll" && environment == "yon";
                !container_marker && !yon_needs_all
            })
            .collect()
    }

    pub(super) fn declarations(
        input: &Input<'_>,
        skip_runtime: bool,
    ) -> Result<Vec<Type>, Failure> {
        let mut result = Vec::new();
        let mut names = BTreeSet::new();
        let mut offset = 0;
        for line in input.code.split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            let Some(rest) = line.strip_prefix("class ") else {
                continue;
            };
            let name = Self::word(rest);
            if skip_runtime && is_reserved_type_name(name) {
                continue;
            }
            if !super::super::identifier(name) || is_reserved_type_name(name) {
                return Err(input.failure(start, "Behavior type requires a safe explicit name outside the reserved Behavior API namespace"));
            }
            if !names.insert(name.to_owned()) {
                return Err(input.failure(start, "Duplicate top-level behavior type name"));
            }
            let body = offset..Self::class_end(&input.code, offset);
            result.push(Self::behavior_type(input, name, start, body));
            if result.len() > 128 {
                return Err(input.failure(start, "Behavior module exceeds 128 types"));
            }
        }
        Ok(result)
    }

    pub(super) fn method_owner(input: &Input<'_>, types: &[Type], at: usize) -> Option<usize> {
        types.iter().position(|ty| {
            at >= ty.body.start
                && at < ty.body.end
                && Self::member_indent(&input.code, ty.body.clone())
                    == Some(Self::indent(&input.code, at))
        })
    }

    pub(super) fn parse_method(input: &Input<'_>, at: usize, ty: &Type) -> Result<Method, Failure> {
        let line_end = Self::line_end(&input.code, at);
        let Some(rest) = input.code[at..line_end].strip_prefix("def ") else {
            return Err(input.failure(at, "Behavior methods must be named concrete declarations"));
        };
        let is_static = rest.starts_with("self.");
        let rest = rest.strip_prefix("self.").unwrap_or(rest);
        let name = Self::word(rest);
        let after_name = line_end - (rest.len() - name.len());
        if !super::super::identifier(name) || input.code[after_name..].starts_with(['?', '!', '='])
        {
            return Err(input.failure(at, "Behavior methods must be named concrete declarations"));
        }
        let (list, signature_end) = Self::parameter_list(input, at, after_name, line_end)?;
        let comments = Yard::comments_before(input.source, at);
        let parameters = Self::parameters(input, at, &input.code[list], &comments)?;
        let body = Self::body(input, at, signature_end, ty)?;
        Ok(Method {
            name: name.to_owned(),
            parameters,
            signature: at..signature_end,
            body,
            return_type: Yard::return_type(&comments),
            asynchronous: false,
            is_static,
            declared: false,
            metadata: serde_json::Map::new(),
        })
    }

    pub(super) fn test_cases(input: &Input<'_>, ty: &Type) -> Vec<String> {
        let mut names = Vec::new();
        let member_indent = Self::member_indent(&input.code, ty.body.clone());
        for line in input.code[ty.body.clone()].lines() {
            let text = line.trim_start();
            if Some(line.len() - text.len()) != member_indent {
                continue;
            }
            let Some(rest) = text.strip_prefix("def ") else {
                continue;
            };
            let name = Self::word(rest);
            let is_case = name.starts_with("test") && super::super::identifier(name);
            if is_case && !names.iter().any(|known| known == name) {
                names.push(name.to_owned());
            }
        }
        names
    }

    fn behavior_type(input: &Input<'_>, name: &str, start: usize, body: Range<usize>) -> Type {
        Type {
            name: name.to_owned(),
            declaration: start,
            keyword: start..start + "class".len(),
            declared: false,
            fields: Self::fields(input, body.clone()),
            body,
            methods: Vec::new(),
            configuration: false,
            cache: None,
            entity: None,
            table: None,
            constructor: None,
            repository: None,
            policies: serde_json::Map::new(),
        }
    }

    fn fields(input: &Input<'_>, body: Range<usize>) -> Vec<Field> {
        let mut fields = Vec::new();
        let member_indent = Self::member_indent(&input.code, body.clone());
        let mut offset = body.start;
        for line in input.code[body].split_inclusive('\n') {
            let indent = line.len() - line.trim_start().len();
            let start = offset + indent;
            offset += line.len();
            if Some(indent) != member_indent {
                continue;
            }
            let text = line.trim();
            let Some(symbols) = Self::FIELD_WORDS
                .iter()
                .find_map(|word| text.strip_prefix(word))
            else {
                continue;
            };
            let type_source = Yard::return_type(&Yard::comments_before(input.source, start));
            for symbol in symbols.split(',') {
                let Some(name) = symbol.trim().strip_prefix(':') else {
                    continue;
                };
                if super::super::identifier(name) {
                    fields.push(Field {
                        name: name.to_owned(),
                        type_source: type_source.clone(),
                        uninitialized: true,
                        initializer: None,
                        is_static: false,
                    });
                }
            }
        }
        fields
    }

    fn parameter_list(
        input: &Input<'_>,
        at: usize,
        after_name: usize,
        line_end: usize,
    ) -> Result<(Range<usize>, usize), Failure> {
        if input.code[after_name..].starts_with('(') {
            let close = super::super::matching(&input.code, after_name, b'(', b')')
                .ok_or_else(|| input.failure(at, "Behavior method parameters are unbalanced"))?;
            return Ok((after_name + 1..close, close + 1));
        }
        let rest = input.code[after_name..line_end].trim();
        if !rest.is_empty() && !rest.starts_with(';') {
            return Err(input.failure(at, Self::NEEDS_PARENTHESES));
        }
        Ok((after_name..after_name, after_name))
    }

    fn parameters(
        input: &Input<'_>,
        at: usize,
        list: &str,
        comments: &str,
    ) -> Result<Vec<Parameter>, Failure> {
        let mut parameters = Vec::new();
        for part in super::split_parameters(list) {
            let plain = !part.contains(['=', ':', '*', '&']);
            if !plain {
                return Err(input.failure(at, Self::PLAIN_PARAMETERS));
            }
            if !super::super::identifier(part) {
                return Err(input.failure(at, "Invalid behavior parameter name"));
            }
            if part.starts_with("__behavior") {
                return Err(input.failure(at, "Behavior parameter uses a compiler-reserved name"));
            }
            let written = Yard::parameter_type(comments, part);
            let type_source = if written.is_empty() {
                Self::UNTYPED.to_owned()
            } else {
                written
            };
            parameters.push(Parameter {
                name: part.to_owned(),
                type_source,
            });
        }
        if parameters.len() > 32 {
            return Err(input.failure(at, "Behavior methods exceed 32 parameters"));
        }
        Ok(parameters)
    }

    fn body(
        input: &Input<'_>,
        at: usize,
        signature_end: usize,
        ty: &Type,
    ) -> Result<Range<usize>, Failure> {
        let line_end = Self::line_end(&input.code, signature_end);
        let rest = &input.code[signature_end..line_end];
        if rest.trim_start().starts_with('=') {
            return Err(input.failure(
                at,
                "Behavior methods require a block body, not an abstract or expression body",
            ));
        }
        if let Some(inline) = rest.trim_end().strip_suffix("end")
            && inline.trim_start().starts_with(';')
        {
            let start = signature_end + rest.find(';').map_or(0, |semicolon| semicolon + 1);
            return Ok(start..signature_end + inline.len());
        }
        let indent = Self::indent(&input.code, at);
        let mut offset = line_end;
        for line in input.code[line_end..].split_inclusive('\n') {
            let text = line.trim();
            let line_indent = line.len() - line.trim_start().len();
            if text == "end" && line_indent == indent {
                let end = offset + indent;
                if end > ty.body.end {
                    break;
                }
                return Ok(signature_end..end);
            }
            offset += line.len();
        }
        Err(input.failure(at, "Behavior method has no matching `end` in its class"))
    }

    fn class_end(code: &str, body_start: usize) -> usize {
        let mut offset = body_start;
        for line in code[body_start..].split_inclusive('\n') {
            if line.trim_end() == "end" {
                return offset;
            }
            offset += line.len();
        }
        code.len()
    }

    fn member_indent(code: &str, body: Range<usize>) -> Option<usize> {
        code[body].lines().find_map(|line| {
            let text = line.trim_start();
            (!text.is_empty()).then_some(line.len() - text.len())
        })
    }

    fn indent(code: &str, at: usize) -> usize {
        at - code[..at].rfind('\n').map_or(0, |newline| newline + 1)
    }

    fn line_end(code: &str, at: usize) -> usize {
        code[at..]
            .find('\n')
            .map_or(code.len(), |newline| at + newline)
    }

    fn word(text: &str) -> &str {
        let length = text
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        &text[..length]
    }
}
