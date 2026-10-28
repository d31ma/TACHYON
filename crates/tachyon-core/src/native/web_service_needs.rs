use super::registry::NativeCompanionInput;
use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::project::NativeCompanion;
use crate::tac_ruby::NeededServices;
use crate::template::required_fields::RequiredFields;
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct WebServiceNeeds {
    language: &'static str,
    consequence: &'static str,
    constructor_inputs: fn(&str, &str, &str) -> Vec<String>,
}

impl WebServiceNeeds {
    const CODE: u16 = 1406;
    const RUST: Self = Self {
        language: "Rust",
        consequence: "the field would hold its own copy of that service, not the one that the page and its other services use",
        constructor_inputs: Self::rust_constructor_inputs,
    };
    const KOTLIN: Self = Self {
        language: "Kotlin",
        consequence: "the page builds each Kotlin browser service on its own, so it has no instance of that service to fill the field with",
        constructor_inputs: Self::no_constructor_inputs,
    };
    const SWIFT: Self = Self {
        language: "Swift",
        consequence: "the page builds each Swift browser service on its own, so it has no instance of that service to fill the field with",
        constructor_inputs: Self::no_constructor_inputs,
    };

    pub(super) fn refuse(selected: &[&NativeCompanionInput]) -> Result<(), Failure> {
        let services = Self::services(selected);
        selected
            .iter()
            .filter_map(|item| Self::of(item).map(|needs| (needs, item)))
            .try_for_each(|(needs, item)| needs.refuse_service(item, &services))
    }

    fn of(item: &NativeCompanionInput) -> Option<Self> {
        match (item.language, item.web_assembly) {
            (NativeCompanion::Rust, true) => Some(Self::RUST),
            (NativeCompanion::Kotlin, true) => Some(Self::KOTLIN),
            (NativeCompanion::Swift, true) => Some(Self::SWIFT),
            _ => None,
        }
    }

    fn services(selected: &[&NativeCompanionInput]) -> BTreeSet<String> {
        let mut services = selected
            .first()
            .map(|item| NeededServices::service_names(&item.snapshot_root))
            .unwrap_or_default();
        services.extend(selected.iter().map(|item| item.class.clone()));
        services
    }

    fn rust_constructor_inputs(source: &str, path: &str, class: &str) -> Vec<String> {
        crate::companion::RustGraph::constructor_inputs(source, path, class).unwrap_or_default()
    }

    fn no_constructor_inputs(_: &str, _: &str, _: &str) -> Vec<String> {
        Vec::new()
    }

    fn refuse_service(
        &self,
        item: &NativeCompanionInput,
        services: &BTreeSet<String>,
    ) -> Result<(), Failure> {
        let source = super::registry::read_captured_source(&item.source)?;
        let relative = item
            .source
            .strip_prefix(&item.snapshot_root)
            .unwrap_or(&item.source)
            .to_string_lossy()
            .replace('\\', "/");
        let mut types = RequiredFields::filled_types(Path::new(&relative), &source, &item.class);
        types.extend((self.constructor_inputs)(&source, &relative, &item.class));
        let needed = types
            .iter()
            .flat_map(|type_name| type_name.split(Self::separates))
            .filter(|word| services.contains(*word))
            .find(|word| *word != item.class);
        needed.map_or(Ok(()), |service| {
            Err(self.failure(&relative, &source, &item.class, service))
        })
    }

    fn separates(letter: char) -> bool {
        [letter.is_alphanumeric(), letter == '_'] == [false, false]
    }

    fn failure(&self, relative: &str, source: &str, class: &str, service: &str) -> Failure {
        let code = crate::annotation_layout::masked_source(Path::new(relative), source)
            .unwrap_or_else(|_| source.to_owned());
        let declared = RequiredFields::class_body(&code, class).map_or(0, |(at, _)| at);
        let start = code[declared..]
            .find(service)
            .map_or(0, |found| declared + found);
        let language = self.language;
        let consequence = self.consequence;
        let supporting = NeededServices::SUPPORTING;
        Failure::one(diagnostic(
            Self::CODE,
            format!(
                "{class} needs {service}, another Tac service, in a field. A {language} browser service cannot need another service yet: {consequence}."
            ),
            Some(format!(
                "A {language} Needs field takes a provider that the {language} browser build supplies, such as an HTTPClient or a Delegate. Let the view call {service} itself, publish an event that {service} handles in an OnEvent method, or write both services in a Tac language that can need another service: {supporting}."
            )),
            source_span(relative, start, start + service.len()),
        ))
    }
}
