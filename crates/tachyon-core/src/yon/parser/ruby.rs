use super::{Input, ManagedType, Signature, identifier, matching};
use crate::Failure;
use crate::ascii_name::AsciiName;
use crate::ruby_source::Yard;
use crate::yon::plan::{Injection, ParameterSource, RouteParameter};
use std::ops::Range;

pub(super) struct RubyMethod;

impl RubyMethod {
    const CLASS_METHOD: &'static str = "Annotated Ruby methods are instance methods: write `def get_orders`, not `def self.get_orders`";
    const NEEDS_PARENTHESES: &'static str = "Put the parameters of an annotated Ruby method in parentheses, for example `def get_order(id)`";
    const NAME_SUFFIXES: [u8; 3] = *b"?!=";
    const NEEDS_ALL_MARKERS: [&'static str; 2] = ["#[NeedsAll]", "#[NeedsAll()]"];
    const FIELD_DECLARATIONS: [&'static str; 3] =
        ["attr_reader ", "attr_accessor ", "attr_writer "];

    pub(super) fn signature(input: &Input<'_>, start: usize) -> Result<Signature, Failure> {
        let (name, after_name) = Self::name(input, start)?;
        let parameters = Self::parameters(input, start, after_name)?;
        let end = Self::signature_end(input, &parameters, after_name);
        Ok(Signature {
            name,
            end,
            generator: false,
        })
    }

    fn signature_end(input: &Input<'_>, parameters: &Range<usize>, after_name: usize) -> usize {
        if parameters.is_empty() && input.code.as_bytes().get(parameters.end) != Some(&b')') {
            after_name
        } else {
            parameters.end + 1
        }
    }

    pub(super) fn parameter_list(input: &Input<'_>, start: usize) -> Result<Range<usize>, Failure> {
        let (_, after_name) = Self::name(input, start)?;
        Self::parameters(input, start, after_name)
    }

    fn require(condition: bool, refusal: impl FnOnce() -> Failure) -> Result<(), Failure> {
        condition.then_some(()).ok_or_else(refusal)
    }

    fn name(input: &Input<'_>, start: usize) -> Result<(String, usize), Failure> {
        let line_end = input.code[start..]
            .find('\n')
            .map_or(input.code.len(), |at| start + at);
        let line = &input.code[start..line_end];
        let rest = line
            .strip_prefix("def ")
            .ok_or_else(|| input.failure(start, "An annotated Ruby method starts with `def`"))?
            .trim_start();
        Self::require(!rest.starts_with("self."), || {
            input.failure(start, Self::CLASS_METHOD)
        })?;
        let length = AsciiName::leading(rest).len();
        let name = &rest[..length];
        let suffix = rest.as_bytes().get(length).copied();
        Self::require(Self::is_plain_name(name, suffix), || {
            input.failure(start, "Annotated methods require non-reserved ASCII identifiers, without generics or computed names")
        })?;
        let after_name = line_end - (rest.len() - length);
        Ok((name.to_owned(), after_name))
    }

    fn is_plain_name(name: &str, suffix: Option<u8>) -> bool {
        let marked = suffix.is_some_and(|byte| Self::NAME_SUFFIXES.contains(&byte));
        identifier(name) && !marked
    }

    fn parameters(
        input: &Input<'_>,
        start: usize,
        after_name: usize,
    ) -> Result<Range<usize>, Failure> {
        if input.code[after_name..].starts_with('(') {
            Self::parenthesized_parameters(input, start, after_name)
        } else {
            Self::bare_parameters(input, start, after_name)
        }
    }

    fn parenthesized_parameters(
        input: &Input<'_>,
        start: usize,
        after_name: usize,
    ) -> Result<Range<usize>, Failure> {
        let closing = matching(&input.code, after_name, b'(', b')')
            .ok_or_else(|| input.failure(start, "Unbalanced annotated method parameters"))?;
        Ok(after_name + 1..closing)
    }

    fn bare_parameters(
        input: &Input<'_>,
        start: usize,
        after_name: usize,
    ) -> Result<Range<usize>, Failure> {
        let rest = &input.code[after_name..];
        let line_rest = rest.split('\n').next().unwrap_or("").trim();
        let no_parameters = [line_rest.is_empty(), line_rest.starts_with(';')].contains(&true);
        Self::require(no_parameters, || {
            input.failure(start, Self::NEEDS_PARENTHESES)
        })?;
        Ok(after_name..after_name)
    }

    pub(super) fn body(input: &Input<'_>, start: usize, signature_end: usize) -> Range<usize> {
        let line_start = input.code[..start].rfind('\n').map_or(0, |at| at + 1);
        let indent = start - line_start;
        let opening = input.code[signature_end..]
            .find('\n')
            .map_or(input.code.len(), |at| signature_end + at + 1);
        let length: usize = input.code[opening..]
            .split_inclusive('\n')
            .take_while(|line| Self::is_inside_body(line, indent))
            .map(str::len)
            .sum();
        opening..opening + length
    }

    fn is_inside_body(line: &str, indent: usize) -> bool {
        line.trim().is_empty() || line.len() - line.trim_start().len() > indent
    }

    pub(super) fn add_field_injections(
        input: &Input<'_>,
        types: &mut [ManagedType],
    ) -> Result<(), Failure> {
        types
            .iter_mut()
            .try_for_each(|managed| Self::inject_fields(input, managed))
    }

    fn inject_fields(input: &Input<'_>, managed: &mut ManagedType) -> Result<(), Failure> {
        let needs_all = Self::needs_all(input, managed.body.start);
        let fields = NeededFields::read(input, &managed.body, needs_all)?;
        let marked_twice =
            [fields.is_empty(), managed.provider.dependencies.is_empty()] == [false, false];
        Self::require(!marked_twice, || {
            input.failure(
                managed.declaration,
                "Use #[Needs] on attr_reader fields or #[Needs(A, B)] on the class, not both",
            )
        })?;
        for (field, provider) in fields {
            managed.provider.dependencies.push(provider);
            managed.provider.injections.push(Injection {
                field,
                pointer: false,
            });
        }
        Ok(())
    }

    pub(super) fn validate_constructors(
        input: &Input<'_>,
        types: &[ManagedType],
    ) -> Result<(), Failure> {
        types
            .iter()
            .filter(|managed| managed.provider.injections.is_empty())
            .try_for_each(|managed| Self::check_constructor(input, managed))
    }

    fn check_constructor(input: &Input<'_>, managed: &ManagedType) -> Result<(), Failure> {
        let count = super::helper_providers::HelperProviders::count(input, managed)?;
        let needed = managed.provider.dependencies.len();
        Self::require(count == needed, || {
            input.failure(
                managed.declaration,
                format!(
                    "{} initialize takes {count} arguments but #[Needs] names {needed} providers",
                    managed.provider.name
                ),
            )
        })
    }

    fn needs_all(input: &Input<'_>, body_start: usize) -> bool {
        input.code[..body_start]
            .lines()
            .rev()
            .map(str::trim)
            .skip_while(|line| !line.starts_with("#["))
            .take_while(|line| line.starts_with("#["))
            .any(|line| Self::NEEDS_ALL_MARKERS.contains(&line))
    }

    const FIELD_NEEDS: &'static str = "In a Ruby class, #[Needs] stands alone above one `attr_reader :field` line; its `# @return [OrderStore]` type comment names the provider, or without one the field name does, for example `attr_reader :order_store` for OrderStore";

    fn documented_provider(input: &Input<'_>, at: usize) -> Option<String> {
        let written = Yard::return_type(&Yard::comments_before(input.source, at));
        identifier(&written).then_some(written)
    }

    fn reader_field(text: &str) -> Option<String> {
        Self::field_name(text.strip_prefix("attr_reader :")?)
    }

    fn declared_fields(text: &str) -> Option<Vec<String>> {
        let names = Self::FIELD_DECLARATIONS
            .iter()
            .find_map(|prefix| text.strip_prefix(prefix))?;
        names
            .split(',')
            .map(|name| Self::field_name(name.trim().strip_prefix(':')?))
            .collect()
    }

    fn field_name(name: &str) -> Option<String> {
        let valid = !name.is_empty() && name.bytes().all(Self::is_field_byte);
        valid.then(|| name.to_owned())
    }

    fn is_field_byte(byte: u8) -> bool {
        [
            byte.is_ascii_lowercase(),
            byte.is_ascii_digit(),
            byte == b'_',
        ]
        .contains(&true)
    }

    fn named_provider(input: &Input<'_>, at: usize, length: usize) -> Result<String, Failure> {
        let written = &input.source[at..at + length];
        let inside = written
            .strip_prefix("#[Named(")
            .and_then(|rest| rest.strip_suffix(")]"))
            .map(str::trim)
            .unwrap_or_default();
        let name = inside.trim_matches('"');
        let quoted_name = [inside.len() == name.len() + 2, identifier(name)] == [true, true];
        quoted_name.then(|| name.to_owned()).ok_or_else(|| {
            input.failure(
                at,
                "#[Named] takes one provider name in quotes, for example #[Named(\"OrderStore\")]",
            )
        })
    }

    fn provider_name(field: &str) -> String {
        field.split('_').map(Self::capitalized).collect()
    }

    fn capitalized(part: &str) -> String {
        let mut characters = part.chars();
        let first = characters.next().map(|first| first.to_ascii_uppercase());
        first.into_iter().chain(characters).collect()
    }

    pub(super) fn read_types(documentation: &str, parameters: &mut [RouteParameter]) -> String {
        parameters
            .iter_mut()
            .filter(|parameter| parameter.source != ParameterSource::Request)
            .for_each(|parameter| {
                parameter.type_text = Yard::parameter_type(documentation, &parameter.code_name);
            });
        Yard::return_type(documentation)
    }
}

struct NeededFields {
    needs_all: bool,
    fields: Vec<(String, String)>,
    marked: Option<usize>,
    named: Option<String>,
}

impl NeededFields {
    const NEEDS_MARKERS: [&'static str; 2] = ["#[Needs]", "#[Needs()]"];

    fn read(
        input: &Input<'_>,
        body: &Range<usize>,
        needs_all: bool,
    ) -> Result<Vec<(String, String)>, Failure> {
        let mut needed = Self {
            needs_all,
            fields: Vec::new(),
            marked: None,
            named: None,
        };
        let mut offset = body.start;
        for line in input.code[body.clone()].split_inclusive('\n') {
            let at = offset + line.len() - line.trim_start().len();
            offset += line.len();
            needed.read_line(input, line.trim(), at)?;
        }
        needed.finish(input)
    }

    fn read_line(&mut self, input: &Input<'_>, text: &str, at: usize) -> Result<(), Failure> {
        if text.is_empty() {
            Ok(())
        } else if Self::NEEDS_MARKERS.contains(&text) {
            self.marked = Some(at);
            Ok(())
        } else if text.starts_with("#[Needs(") {
            Err(input.failure(at, RubyMethod::FIELD_NEEDS))
        } else if text.starts_with("#[Named(") {
            self.named = Some(RubyMethod::named_provider(input, at, text.len())?);
            Ok(())
        } else if let Some(marked) = self.marked.take() {
            self.add_marked_field(input, text, at, marked)
        } else {
            self.add_declared_fields(input, text, at)
        }
    }

    fn add_marked_field(
        &mut self,
        input: &Input<'_>,
        text: &str,
        at: usize,
        marked: usize,
    ) -> Result<(), Failure> {
        let field = RubyMethod::reader_field(text)
            .ok_or_else(|| input.failure(marked, RubyMethod::FIELD_NEEDS))?;
        let provider = self
            .named
            .take()
            .or_else(|| RubyMethod::documented_provider(input, at))
            .unwrap_or_else(|| RubyMethod::provider_name(&field));
        self.fields.push((field, provider));
        Ok(())
    }

    fn add_declared_fields(
        &mut self,
        input: &Input<'_>,
        text: &str,
        at: usize,
    ) -> Result<(), Failure> {
        let declared = self
            .needs_all
            .then(|| RubyMethod::declared_fields(text))
            .flatten();
        declared.map_or(Ok(()), |declared| self.add_all(input, at, declared))?;
        self.named = None;
        Ok(())
    }

    fn add_all(
        &mut self,
        input: &Input<'_>,
        at: usize,
        declared: Vec<String>,
    ) -> Result<(), Failure> {
        let single = declared.len() == 1;
        let provider = self
            .named
            .take()
            .or_else(|| RubyMethod::documented_provider(input, at).filter(|_| single));
        let shared_provider = [provider.is_some(), !single] == [true, true];
        RubyMethod::require(!shared_provider, || {
            input.failure(at, RubyMethod::FIELD_NEEDS)
        })?;
        for field in declared {
            let provider = provider
                .clone()
                .unwrap_or_else(|| RubyMethod::provider_name(&field));
            self.fields.push((field, provider));
        }
        Ok(())
    }

    fn finish(self, input: &Input<'_>) -> Result<Vec<(String, String)>, Failure> {
        match self.marked {
            Some(marked) => Err(input.failure(marked, RubyMethod::FIELD_NEEDS)),
            None => Ok(self.fields),
        }
    }
}
