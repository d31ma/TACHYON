use super::{
    Annotation, Declaration, Input, ManagedType, annotation, constructor_count, declared_type,
    masked_code, rust_type_bodies, type_body,
};
use crate::Failure;
use crate::failure::diagnostic;
use crate::yon::manual_helpers::HelperSource;
use crate::yon::plan::{Provider, Role, Scope};
use std::collections::BTreeSet;
use std::ops::Range;
use std::path::Path;

pub(super) struct HelperSources;

impl HelperSources {
    pub(super) fn read(path: &Path, source: &str) -> Result<Vec<HelperSource>, Failure> {
        let input = Self::helper_input(path, source)?;
        let declared = Self::helper_raw_declarations(&input)?;
        Self::helper_declarations(&input, declared)
    }

    fn helper_input<'a>(path: &'a Path, source: &'a str) -> Result<Input<'a>, Failure> {
        Self::helper_source_limit(source)?;
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        let mask = crate::lexical::code_mask(extension, source);
        let input = Input {
            path,
            source,
            code: masked_code(source, mask.code),
            extension: crate::groovy_source::GroovySource::syntax(extension),
            manual_helpers: BTreeSet::new(),
        };
        Self::helper_mask_error(&input, mask.error)?;
        Ok(input)
    }

    fn helper_source_limit(source: &str) -> Result<(), Failure> {
        if source.len() > Input::MAX_SOURCE {
            return Err(Failure::one(diagnostic(
                2015,
                "Yon helper source exceeds 1 MiB",
                None,
                None,
            )));
        }
        Ok(())
    }

    fn helper_mask_error(input: &Input<'_>, error: Option<&str>) -> Result<(), Failure> {
        if let Some(error) = error {
            return Err(input.failure(0, error));
        }
        Ok(())
    }

    fn helper_raw_declarations(input: &Input<'_>) -> Result<Vec<Declaration>, Failure> {
        let mut declared = Vec::new();
        let mut pending = Vec::new();
        let mut offset = 0;
        for line in input.code.split_inclusive('\n') {
            Self::helper_raw_line(input, line, offset, &mut pending, &mut declared)?;
            offset += line.len();
        }
        Ok(declared)
    }

    fn helper_raw_line(
        input: &Input<'_>,
        line: &str,
        offset: usize,
        pending: &mut Vec<Annotation>,
        declared: &mut Vec<Declaration>,
    ) -> Result<(), Failure> {
        (!line.trim().is_empty())
            .then_some(line)
            .map(|line| Self::helper_raw_content(input, line, offset, pending, declared))
            .transpose()
            .map(|_| ())
    }

    fn helper_raw_content(
        input: &Input<'_>,
        line: &str,
        offset: usize,
        pending: &mut Vec<Annotation>,
        declared: &mut Vec<Declaration>,
    ) -> Result<(), Failure> {
        let start = offset + line.len() - line.trim_start().len();
        let end = offset + line.trim_end_matches(['\r', '\n']).len();
        let marker = Self::helper_raw_annotation(input, start, end)?;
        if let Some(marker) = marker {
            pending.push(marker);
            return Ok(());
        }
        Self::helper_raw_type(line, start, pending, declared);
        Ok(())
    }

    fn helper_raw_type(
        line: &str,
        start: usize,
        pending: &mut Vec<Annotation>,
        declared: &mut Vec<Declaration>,
    ) {
        (!line.trim_start().starts_with(['@', '[', '#']))
            .then(|| Self::helper_raw_pending(start, pending, declared));
    }

    fn helper_raw_pending(
        start: usize,
        pending: &mut Vec<Annotation>,
        declared: &mut Vec<Declaration>,
    ) {
        if pending.is_empty() {
            return;
        }
        declared.push(Declaration {
            annotations: std::mem::take(pending),
            start,
        });
    }

    fn helper_raw_annotation(
        input: &Input<'_>,
        start: usize,
        end: usize,
    ) -> Result<Option<Annotation>, Failure> {
        Self::helper_raw_needs_all(input, start, end)
            .map_or_else(|| annotation(input, start, end), |marker| Ok(Some(marker)))
    }

    fn helper_raw_needs_all(input: &Input<'_>, start: usize, end: usize) -> Option<Annotation> {
        let name = input.code[start..end]
            .trim_start_matches(['@', '[', '#', '<'])
            .split(Self::helper_name_separator)
            .next()?;
        let plain = name
            .rsplit(':')
            .next()?
            .replace('_', "")
            .to_ascii_lowercase();
        if plain != "needsall" {
            return None;
        }
        Some(Annotation {
            name: plain,
            arguments: String::new(),
            span: start..end,
        })
    }

    fn helper_name_separator(character: char) -> bool {
        [
            !character.is_ascii_alphanumeric(),
            !['_', ':'].contains(&character),
        ]
        .contains(&false)
            == false
    }

    fn helper_declarations(
        input: &Input<'_>,
        declared: Vec<Declaration>,
    ) -> Result<Vec<HelperSource>, Failure> {
        declared
            .into_iter()
            .filter(Self::helper_marker)
            .filter_map(|item| Self::helper_source(input, &item).transpose())
            .collect()
    }

    fn helper_marker(item: &Declaration) -> bool {
        item.annotations
            .iter()
            .any(|marker| marker.name == "helper")
    }

    fn helper_source(
        input: &Input<'_>,
        item: &Declaration,
    ) -> Result<Option<HelperSource>, Failure> {
        declared_type(input, item)
            .map(|(name, declaration)| Self::helper_named_source(input, item, name, declaration))
            .transpose()
    }

    fn helper_named_source(
        input: &Input<'_>,
        item: &Declaration,
        name: String,
        declaration: usize,
    ) -> Result<HelperSource, Failure> {
        let body = type_body(input, item.start)?;
        let managed = item
            .annotations
            .iter()
            .any(|marker| ["needs", "needsall", "lifetime"].contains(&marker.name.as_str()));
        let implementations = Self::helper_implementations(input, &name, &body, declaration)?;
        let zero_argument = Self::helper_zero_argument(input, &name, &body, declaration)?;
        Ok(HelperSource {
            name,
            body,
            declaration,
            managed,
            implementations,
            zero_argument,
        })
    }

    fn helper_zero_argument(
        input: &Input<'_>,
        name: &str,
        body: &Range<usize>,
        declaration: usize,
    ) -> Result<bool, Failure> {
        let managed = ManagedType {
            provider: Provider {
                name: name.to_owned(),
                role: Role::Component,
                scope: Scope::Singleton,
                dependencies: Vec::new(),
                injections: Vec::new(),
            },
            body: body.clone(),
            declaration,
        };
        let count = constructor_count(input, &managed).ok();
        Self::helper_zero_fields(input, body, count)
    }

    fn helper_zero_fields(
        input: &Input<'_>,
        body: &Range<usize>,
        count: Option<usize>,
    ) -> Result<bool, Failure> {
        match (input.extension, count) {
            ("rb", Some(0)) => Ok(true),
            (_, Some(0)) => {
                crate::yon::required_args::required_fields(input.path, input.source, body.clone())
                    .map(|fields| fields.is_empty())
            }
            _ => Ok(false),
        }
    }

    fn helper_implementations(
        input: &Input<'_>,
        name: &str,
        body: &Range<usize>,
        declaration: usize,
    ) -> Result<Vec<Range<usize>>, Failure> {
        match input.extension {
            "go" => Ok(Self::helper_go_implementations(input, name)),
            "rs" => Self::helper_rust_implementations(input, name, body, declaration),
            _ => Ok(Vec::new()),
        }
    }

    fn helper_rust_implementations(
        input: &Input<'_>,
        name: &str,
        body: &Range<usize>,
        declaration: usize,
    ) -> Result<Vec<Range<usize>>, Failure> {
        let managed = ManagedType {
            provider: Provider {
                name: name.to_owned(),
                role: Role::Component,
                scope: Scope::Singleton,
                dependencies: Vec::new(),
                injections: Vec::new(),
            },
            body: body.clone(),
            declaration,
        };
        rust_type_bodies(input, &managed)
    }

    fn helper_go_implementations(input: &Input<'_>, name: &str) -> Vec<Range<usize>> {
        let methods = crate::go_source::methods(&input.code);
        crate::go_source::methods_of(&methods, name)
            .into_iter()
            .map(|method| Self::helper_go_marker_start(input, method.start)..method.body.end)
            .collect()
    }

    fn helper_go_marker_start(input: &Input<'_>, start: usize) -> usize {
        let beginning = input.code[..start]
            .rfind('\n')
            .map_or(0, |offset| offset + 1);
        let markers: usize = input.code[..beginning]
            .split_inclusive('\n')
            .rev()
            .take_while(|line| Self::helper_go_marker_line(line))
            .map(str::len)
            .sum();
        beginning - markers
    }

    fn helper_go_marker_line(line: &str) -> bool {
        let trimmed = line.trim();
        [trimmed.is_empty(), trimmed.starts_with('@')].contains(&true)
    }
}
