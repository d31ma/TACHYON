use super::{Input, ManagedType, Signature, constructor_count, identifier, matching};
use crate::Failure;
use crate::ruby_source::Yard;
use crate::yon::plan::{Injection, ParameterSource, RouteParameter};
use std::ops::Range;

pub(super) struct RubyMethod;

impl RubyMethod {
    const CLASS_METHOD: &'static str = "Annotated Ruby methods are instance methods: write `def get_orders`, not `def self.get_orders`";
    const NEEDS_PARENTHESES: &'static str = "Put the parameters of an annotated Ruby method in parentheses, for example `def get_order(id)`";

    pub(super) fn signature(input: &Input<'_>, start: usize) -> Result<Signature, Failure> {
        let (name, after_name) = Self::name(input, start)?;
        let parameters = Self::parameters(input, start, after_name)?;
        let end =
            if parameters.is_empty() && input.code.as_bytes().get(parameters.end) != Some(&b')') {
                after_name
            } else {
                parameters.end + 1
            };
        Ok(Signature {
            name,
            end,
            generator: false,
        })
    }

    pub(super) fn parameter_list(input: &Input<'_>, start: usize) -> Result<Range<usize>, Failure> {
        let (_, after_name) = Self::name(input, start)?;
        Self::parameters(input, start, after_name)
    }

    fn name(input: &Input<'_>, start: usize) -> Result<(String, usize), Failure> {
        let line_end = input.code[start..]
            .find('\n')
            .map_or(input.code.len(), |at| start + at);
        let line = &input.code[start..line_end];
        let Some(rest) = line.strip_prefix("def ") else {
            return Err(input.failure(start, "An annotated Ruby method starts with `def`"));
        };
        let rest = rest.trim_start();
        if rest.starts_with("self.") {
            return Err(input.failure(start, Self::CLASS_METHOD));
        }
        let length = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
            .count();
        let name = &rest[..length];
        let suffix = rest.as_bytes().get(length).copied();
        if !identifier(name) || matches!(suffix, Some(b'?' | b'!' | b'=')) {
            return Err(input.failure(start, "Annotated methods require non-reserved ASCII identifiers, without generics or computed names"));
        }
        let after_name = line_end - (rest.len() - length);
        Ok((name.to_owned(), after_name))
    }

    fn parameters(
        input: &Input<'_>,
        start: usize,
        after_name: usize,
    ) -> Result<Range<usize>, Failure> {
        let rest = &input.code[after_name..];
        if rest.starts_with('(') {
            let closing = matching(&input.code, after_name, b'(', b')')
                .ok_or_else(|| input.failure(start, "Unbalanced annotated method parameters"))?;
            return Ok(after_name + 1..closing);
        }
        let line_rest = rest.split('\n').next().unwrap_or("").trim();
        if !line_rest.is_empty() && !line_rest.starts_with(';') {
            return Err(input.failure(start, Self::NEEDS_PARENTHESES));
        }
        Ok(after_name..after_name)
    }

    pub(super) fn body(input: &Input<'_>, start: usize, signature_end: usize) -> Range<usize> {
        let line_start = input.code[..start].rfind('\n').map_or(0, |at| at + 1);
        let indent = start - line_start;
        let opening = input.code[signature_end..]
            .find('\n')
            .map_or(input.code.len(), |at| signature_end + at + 1);
        let mut cursor = opening;
        for line in input.code[opening..].split_inclusive('\n') {
            let text = line.trim();
            if !text.is_empty() && line.len() - line.trim_start().len() <= indent {
                return opening..cursor;
            }
            cursor += line.len();
        }
        opening..input.code.len()
    }

    pub(super) fn add_field_injections(
        input: &Input<'_>,
        types: &mut [ManagedType],
    ) -> Result<(), Failure> {
        for managed in types.iter_mut() {
            let needs_all = Self::needs_all(input, managed.body.start);
            let fields = Self::needed_fields(input, &managed.body, needs_all)?;
            if fields.is_empty() {
                continue;
            }
            if !managed.provider.dependencies.is_empty() {
                return Err(input.failure(
                    managed.declaration,
                    "Use #[Needs] on attr_reader fields or #[Needs(A, B)] on the class, not both",
                ));
            }
            for (field, provider) in fields {
                managed.provider.dependencies.push(provider);
                managed.provider.injections.push(Injection {
                    field,
                    pointer: false,
                });
            }
        }
        Ok(())
    }

    pub(super) fn validate_constructors(
        input: &Input<'_>,
        types: &[ManagedType],
    ) -> Result<(), Failure> {
        for managed in types {
            if !managed.provider.injections.is_empty() {
                continue;
            }
            let count = constructor_count(input, managed)?;
            let needed = managed.provider.dependencies.len();
            if count != needed {
                return Err(input.failure(
                    managed.declaration,
                    format!(
                        "{} initialize takes {count} arguments but #[Needs] names {needed} providers",
                        managed.provider.name
                    ),
                ));
            }
        }
        Ok(())
    }

    fn needs_all(input: &Input<'_>, body_start: usize) -> bool {
        input.code[..body_start]
            .lines()
            .rev()
            .map(str::trim)
            .skip_while(|line| !line.starts_with("#["))
            .take_while(|line| line.starts_with("#["))
            .any(|line| matches!(line, "#[NeedsAll]" | "#[NeedsAll()]"))
    }

    fn needed_fields(
        input: &Input<'_>,
        body: &Range<usize>,
        needs_all: bool,
    ) -> Result<Vec<(String, String)>, Failure> {
        let mut fields = Vec::new();
        let mut needs = None;
        let mut named = None;
        let mut offset = body.start;
        for line in input.code[body.clone()].split_inclusive('\n') {
            let text = line.trim();
            let at = offset + line.len() - line.trim_start().len();
            offset += line.len();
            if text.is_empty() {
                continue;
            }
            if text == "#[Needs]" || text == "#[Needs()]" {
                needs = Some(at);
                continue;
            }
            if text.starts_with("#[Needs(") {
                return Err(input.failure(at, Self::FIELD_NEEDS));
            }
            if text.starts_with("#[Named(") {
                named = Some(Self::named_provider(input, at, text.len())?);
                continue;
            }
            let Some(marked) = needs.take() else {
                if needs_all && let Some(declared) = Self::declared_fields(text) {
                    let provider = named.take();
                    if provider.is_some() && declared.len() != 1 {
                        return Err(input.failure(at, Self::FIELD_NEEDS));
                    }
                    for field in declared {
                        let provider = provider
                            .clone()
                            .unwrap_or_else(|| Self::provider_name(&field));
                        fields.push((field, provider));
                    }
                }
                named = None;
                continue;
            };
            let Some(field) = Self::reader_field(text) else {
                return Err(input.failure(marked, Self::FIELD_NEEDS));
            };
            let provider = named.take().unwrap_or_else(|| Self::provider_name(&field));
            fields.push((field, provider));
        }
        if let Some(marked) = needs {
            return Err(input.failure(marked, Self::FIELD_NEEDS));
        }
        Ok(fields)
    }

    const FIELD_NEEDS: &'static str = "In a Ruby class, #[Needs] stands alone above one `attr_reader :field` line; the field name names the provider, for example `attr_reader :order_store` for OrderStore";

    fn reader_field(text: &str) -> Option<String> {
        let name = text.strip_prefix("attr_reader :")?;
        let valid = !name.is_empty()
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
        valid.then(|| name.to_owned())
    }

    fn declared_fields(text: &str) -> Option<Vec<String>> {
        let names = ["attr_reader ", "attr_accessor ", "attr_writer "]
            .iter()
            .find_map(|prefix| text.strip_prefix(prefix))?;
        names
            .split(',')
            .map(|name| {
                let name = name.trim().strip_prefix(':')?;
                let valid = !name.is_empty()
                    && name.bytes().all(|byte| {
                        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'
                    });
                valid.then(|| name.to_owned())
            })
            .collect()
    }

    fn named_provider(input: &Input<'_>, at: usize, length: usize) -> Result<String, Failure> {
        let written = &input.source[at..at + length];
        let inside = written
            .strip_prefix("#[Named(")
            .and_then(|rest| rest.strip_suffix(")]"))
            .map(str::trim)
            .unwrap_or_default();
        let name = inside.trim_matches('"');
        if inside.len() == name.len() + 2 && identifier(name) {
            return Ok(name.to_owned());
        }
        Err(input.failure(
            at,
            "#[Named] takes one provider name in quotes, for example #[Named(\"OrderStore\")]",
        ))
    }

    fn provider_name(field: &str) -> String {
        let mut name = String::new();
        for part in field.split('_') {
            let mut characters = part.chars();
            if let Some(first) = characters.next() {
                name.push(first.to_ascii_uppercase());
                name.push_str(characters.as_str());
            }
        }
        name
    }

    pub(super) fn read_types(documentation: &str, parameters: &mut [RouteParameter]) -> String {
        for parameter in parameters {
            if parameter.source == ParameterSource::Request {
                continue;
            }
            parameter.type_text = Yard::parameter_type(documentation, &parameter.code_name);
        }
        Yard::return_type(documentation)
    }
}
