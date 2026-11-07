use super::{DelegateForwards, FsharpParameters, SourceLines};
use crate::Failure;
use crate::annotation_rules::{AnnotationRules, NeededField};
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct DelegateBase<'a> {
    pub(super) field: String,
    pub(super) field_at: usize,
    pub(super) service: String,
    pub(super) service_path: &'a Path,
    pub(super) service_source: &'a str,
}

impl<'a> DelegateBase<'a> {
    const CODE: u16 = 1373;
    const NAME_MARKS: [char; 3] = ['$', '!', '?'];
    const HELP: &'static str = "Keep one service field in this Delegate: that service is its base, and the Delegate takes on its methods. Call the other service from the class that needs this Delegate, or give it a Delegate of its own (ADR 0112).";

    pub(super) fn read(
        path: &Path,
        source: &str,
        services: &[(&'a Path, &'a str)],
    ) -> Result<Option<Self>, Failure> {
        let bases: Vec<(NeededField, (&'a Path, &'a str))> =
            AnnotationRules::needed_fields(path, source)
                .into_iter()
                .chain(FsharpParameters::needed(path, source))
                .filter_map(|field| {
                    let service = Self::service_of(source, &field, services)?;
                    Some((field, service))
                })
                .collect();
        let first = bases.first().map(|(_, service)| service.0);
        let second = bases
            .iter()
            .find(|(_, service)| first.is_some_and(|chosen| chosen != service.0));
        second.map_or(Ok(()), |(field, service)| {
            Err(Self::second_base(path, source, (first, field, service.0)))
        })?;
        Ok(bases.first().map(Self::from_field))
    }

    fn from_field(base: &(NeededField, (&'a Path, &'a str))) -> Self {
        let (field, (service_path, service_source)) = base;
        Self {
            field: field.name.trim_matches(Self::NAME_MARKS).to_owned(),
            field_at: field.at,
            service: DelegateForwards::class_name(service_path),
            service_path: *service_path,
            service_source: *service_source,
        }
    }

    fn service_of(
        source: &str,
        field: &NeededField,
        services: &[(&'a Path, &'a str)],
    ) -> Option<(&'a Path, &'a str)> {
        let mut words = Self::words_near(source, field.at);
        words.insert(Self::class_form(&field.name));
        services
            .iter()
            .copied()
            .find(|(path, _)| words.contains(&DelegateForwards::class_name(path)))
    }

    fn words_near(source: &str, at: usize) -> BTreeSet<String> {
        let (start, end) = SourceLines::line_around(source, at);
        let notes: Vec<&str> = source[..start]
            .lines()
            .rev()
            .take_while(|line| SourceLines::is_note(line))
            .collect();
        let mut words = SourceLines::words(&source[start..end]);
        words.extend(notes.iter().flat_map(|line| SourceLines::words(line)));
        words
    }

    fn class_form(name: &str) -> String {
        name.trim_matches(Self::NAME_MARKS)
            .split('_')
            .map(Self::capitalized)
            .collect()
    }

    fn capitalized(word: &str) -> String {
        let mut letters = word.chars();
        letters
            .next()
            .map(|first| first.to_ascii_uppercase().to_string() + letters.as_str())
            .unwrap_or_default()
    }

    fn second_base(
        path: &Path,
        source: &str,
        (first, field, second): (Option<&Path>, &NeededField, &Path),
    ) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let first = first.map(DelegateForwards::class_name).unwrap_or_default();
        let message = format!(
            "{} needs two services, {first} and {}. A Delegate takes on the methods of one service, its base.",
            DelegateForwards::class_name(path),
            DelegateForwards::class_name(second),
        );
        let end = (field.at + field.name.len()).min(source.len());
        Failure::one(diagnostic(
            Self::CODE,
            message,
            Some(Self::HELP.to_owned()),
            source_span(&portable, field.at, end),
        ))
    }
}
