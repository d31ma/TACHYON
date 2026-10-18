use super::java_constructor_provider::JavaConstructorProvider;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) struct JavaNeedOwner<'a> {
    page: &'a [(String, String, usize, bool)],
    owner: &'a str,
    path: &'a Path,
    authored: &'a str,
    qualifiers: BTreeMap<String, String>,
}

impl<'a> JavaNeedOwner<'a> {
    const CODE: u16 = 1406;
    const NOWHERE: &'static [(String, String, usize, bool)] = &[];

    pub(super) fn new(
        page: &'a [(String, String, usize, bool)],
        (owner, path, authored): (&'a str, &'a Path, &'a str),
        qualifiers: BTreeMap<String, String>,
    ) -> Self {
        Self {
            page,
            owner,
            path,
            authored,
            qualifiers,
        }
    }

    pub(super) fn alone(path: &'a Path, authored: &'a str) -> Self {
        Self::new(Self::NOWHERE, ("", path, authored), BTreeMap::new())
    }

    pub(super) fn provider(
        &self,
        kind: &str,
        field: &str,
    ) -> Result<Option<JavaConstructorProvider>, Failure> {
        let named = self.qualifiers.get(field);
        let target = named.map_or(kind, String::as_str);
        let candidates = self.candidates(target);
        let unknown = [candidates.is_empty(), named.is_some()] == [true, true];
        let refused = unknown.then(|| self.unknown(field, target));
        refused.map_or_else(|| self.chosen(&candidates, target, field), Err)
    }

    fn chosen(
        &self,
        candidates: &[&(String, String, usize, bool)],
        target: &str,
        field: &str,
    ) -> Result<Option<JavaConstructorProvider>, Failure> {
        let by_name = candidates.iter().find(|service| service.0 == field);
        let only = (candidates.len() == 1)
            .then(|| candidates.first())
            .flatten();
        let picked = by_name.or(only);
        let undecided = [picked.is_none(), candidates.len() > 1] == [true, true];
        let refused = undecided.then(|| self.undecided(field, target, candidates));
        refused.map_or_else(|| self.provided(picked.copied(), field), Err)
    }

    fn provided(
        &self,
        picked: Option<&(String, String, usize, bool)>,
        field: &str,
    ) -> Result<Option<JavaConstructorProvider>, Failure> {
        let one_per_component = picked.filter(|service| !service.3);
        one_per_component.map_or_else(
            || Ok(picked.map(|service| Self::service(service, field))),
            |service| Err(self.per_component(field, &service.1)),
        )
    }

    fn service(service: &(String, String, usize, bool), field: &str) -> JavaConstructorProvider {
        let (instance, class, index, _) = service;
        JavaConstructorProvider::Service {
            name: String::from(field),
            instance: instance.clone(),
            class: format!(
                "{}.{class}",
                super::java_enclosure::JavaEnclosure::holder(*index)
            ),
        }
    }

    fn candidates(&self, class: &str) -> Vec<&(String, String, usize, bool)> {
        self.page
            .iter()
            .filter(|service| service.1 == class)
            .collect()
    }

    fn unknown(&self, field: &str, target: &str) -> Failure {
        self.refused(
            field,
            &format!(
                "Java service {} names {target} with @Named, and the page has no Java service {target}. Name the class of a Java service on the page.",
                self.owner_class()
            ),
        )
    }

    fn undecided(
        &self,
        field: &str,
        target: &str,
        candidates: &[&(String, String, usize, bool)],
    ) -> Failure {
        let names: Vec<&str> = candidates
            .iter()
            .map(|service| service.0.as_str())
            .collect();
        self.refused(
            field,
            &format!(
                "Java service {} cannot tell which {target} it needs: the page has {}. Name the field after one of them.",
                self.owner_class(),
                names.join(" and ")
            ),
        )
    }

    fn per_component(&self, field: &str, target: &str) -> Failure {
        let owner = self.owner_class();
        self.refused(
            field,
            &format!(
                "Java service {owner} cannot need {target}: {target} has one instance for each component, because it is Lifetime prototype or NeedsAll, so a field cannot name one of them. Make {target} a singleton without properties, or pass the value to a method of {owner}."
            ),
        )
    }

    fn owner_class(&self) -> &str {
        self.path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or(self.owner)
    }

    fn refused(&self, field: &str, message: &str) -> Failure {
        let at = Self::field_at(self.authored, field);
        Failure::one(diagnostic(
            Self::CODE,
            message,
            None,
            source_span(&self.path.to_string_lossy(), at, at + field.len()),
        ))
    }

    fn field_at(authored: &str, field: &str) -> usize {
        authored
            .match_indices(field)
            .map(|(at, _)| at)
            .find(|at| Self::declares(authored, *at, field))
            .unwrap_or_default()
    }

    fn declares(authored: &str, at: usize, field: &str) -> bool {
        let before = authored[..at].chars().next_back();
        let after = authored[at + field.len()..].trim_start().chars().next();
        [
            before.is_some_and(char::is_whitespace),
            [Some(';'), Some('=')].contains(&after),
        ] == [true, true]
    }
}
