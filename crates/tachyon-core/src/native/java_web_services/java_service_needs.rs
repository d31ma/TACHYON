use super::java_code::JavaCode;
use super::java_constructor_provider::JavaConstructorProvider;
use super::java_enclosure::JavaEnclosure;
use super::java_need_owner::JavaNeedOwner;
use super::java_service::JavaService;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub(super) struct JavaServiceNeeds {
    page: Vec<(String, String, usize, bool)>,
}

impl JavaServiceNeeds {
    const CODE: u16 = 1406;
    const QUALIFIED: u8 = b'.';

    pub(super) fn read(
        services: &[(String, String, PathBuf)],
        texts: &[String],
    ) -> Result<Self, Failure> {
        let page = services
            .iter()
            .zip(texts)
            .enumerate()
            .map(|(index, ((instance, class, path), text))| {
                Self::entry(index, (instance, class), path, text)
            })
            .collect::<Result<_, Failure>>()?;
        Ok(Self { page })
    }

    pub(super) fn owner<'a>(
        &'a self,
        instance: &'a str,
        path: &'a Path,
        (authored, lowered, class): (&'a str, &str, &str),
    ) -> Result<JavaNeedOwner<'a>, Failure> {
        let written = crate::behavior::ProviderQualifiers::read(path, lowered, class)?;
        let qualifiers = serde_json::from_value(written).unwrap_or_default();
        Ok(JavaNeedOwner::new(
            &self.page,
            (instance, path, authored),
            qualifiers,
        ))
    }

    pub(super) fn shared(
        services: &[JavaService],
        texts: &[String],
        bases: &BTreeMap<String, Vec<String>>,
    ) -> Result<BTreeSet<String>, Failure> {
        let edges = Self::edges(services, bases);
        services
            .iter()
            .zip(texts)
            .try_for_each(|written| Self::require_open(&edges, written))?;
        Ok(edges.values().flatten().cloned().collect())
    }

    pub(super) fn qualified(
        &self,
        source: &str,
        own: &str,
        providers: &[JavaConstructorProvider],
    ) -> String {
        self.holders(own, providers)
            .iter()
            .fold(source.to_owned(), |text, (plain, class)| {
                Self::qualify(&text, plain, class)
            })
    }

    fn holders(
        &self,
        own: &str,
        providers: &[JavaConstructorProvider],
    ) -> BTreeMap<String, String> {
        let mut classes: BTreeMap<String, String> = self
            .page
            .iter()
            .rev()
            .map(|(_, class, index, _)| {
                (
                    class.clone(),
                    format!("{}.{class}", JavaEnclosure::holder(*index)),
                )
            })
            .collect();
        classes.extend(
            providers
                .iter()
                .filter_map(JavaConstructorProvider::needed)
                .map(|(_, class)| (Self::plain(class).to_owned(), class.to_owned())),
        );
        classes.remove(own);
        classes
    }

    fn entry(
        index: usize,
        (instance, class): (&str, &str),
        path: &Path,
        text: &str,
    ) -> Result<(String, String, usize, bool), Failure> {
        let roles = crate::behavior::ServiceRoles::read(path, text)?;
        let provides = [roles.prototype, roles.required.is_empty()] == [false, true];
        Ok((instance.to_owned(), class.to_owned(), index, provides))
    }

    fn plain(class: &str) -> &str {
        class.rsplit('.').next().unwrap_or(class)
    }

    fn qualify(source: &str, plain: &str, class: &str) -> String {
        let code = JavaCode::new(source);
        let visible = String::from_utf8_lossy(code.bytes()).into_owned();
        let mut text = String::with_capacity(source.len());
        let mut from = 0;
        for (at, _) in visible.match_indices(plain) {
            let replaced = Self::is_reference(code.bytes(), at, plain.len());
            text.push_str(&source[from..at]);
            text.push_str([plain, class][usize::from(replaced)]);
            from = at + plain.len();
        }
        text.push_str(&source[from..]);
        text
    }

    fn is_reference(code: &[u8], at: usize, length: usize) -> bool {
        let before = at.checked_sub(1).and_then(|previous| code.get(previous));
        let after = code.get(at + length);
        let word = |byte: Option<&u8>| {
            byte.is_some_and(|byte| byte.is_ascii_alphanumeric() || b"_$".contains(byte))
        };
        [word(before), before == Some(&Self::QUALIFIED), word(after)] == [false, false, false]
    }

    fn edges(
        services: &[JavaService],
        bases: &BTreeMap<String, Vec<String>>,
    ) -> BTreeMap<String, BTreeSet<String>> {
        services
            .iter()
            .map(|service| {
                let mut needed = Self::needed(service);
                needed.extend(bases.get(&service.instance).into_iter().flatten().cloned());
                (service.instance.clone(), needed)
            })
            .collect()
    }

    fn needed(service: &JavaService) -> BTreeSet<String> {
        service
            .providers
            .iter()
            .filter_map(JavaConstructorProvider::needed)
            .map(|(instance, _)| instance.to_owned())
            .collect()
    }

    fn require_open(
        edges: &BTreeMap<String, BTreeSet<String>>,
        (service, authored): (&JavaService, &String),
    ) -> Result<(), Failure> {
        let needs = edges.get(&service.instance).cloned().unwrap_or_default();
        let circle = needs
            .iter()
            .find(|target| Self::reaches(edges, target, &service.instance));
        circle.map_or(Ok(()), |target| {
            Err(Self::circle(service, authored, target))
        })
    }

    fn reaches(edges: &BTreeMap<String, BTreeSet<String>>, from: &str, wanted: &str) -> bool {
        let mut seen = BTreeSet::new();
        let mut waiting = vec![from.to_owned()];
        while let Some(next) = waiting.pop() {
            let fresh = seen.insert(next.clone());
            let onward = edges.get(&next).filter(|_| fresh);
            waiting.extend(onward.into_iter().flatten().cloned());
        }
        seen.contains(wanted)
    }

    fn circle(service: &JavaService, authored: &str, target: &str) -> Failure {
        let at = authored
            .find(&format!("class {}", service.class))
            .unwrap_or_default();
        Failure::one(diagnostic(
            Self::CODE,
            format!(
                "Java services {} and {target} need each other in a circle. Break the circle with an event: one service publishes it and the other listens with OnEvent.",
                service.instance
            ),
            None,
            source_span(
                &service.path.to_string_lossy(),
                at,
                at + service.class.len(),
            ),
        ))
    }
}
