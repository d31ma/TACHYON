use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::ruby_source::{NeededField, RubyNeeds};
use std::collections::BTreeSet;
use std::path::Path;

pub(crate) struct NeededServices {
    language: &'static str,
    decision: &'static str,
    injected: &'static str,
}

impl NeededServices {
    pub(crate) const RUBY: Self = Self {
        language: "Ruby",
        decision: "ADR 0054",
        injected: "a Store, a Converter, a Helper or an HTTPClient",
    };
    pub(crate) const LUA: Self = Self {
        language: "Lua",
        decision: "ADR 0085",
        injected: "a Store, a Converter, a Helper or an HTTPClient",
    };
    const CODE: u16 = 2015;
    const SERVICES: &'static str = "client/services";
    const SUPPORTING: &'static str = "JavaScript, TypeScript, Go, Python, Dart, C++ or F#";

    pub(crate) fn refuse(
        &self,
        snapshot_root: &Path,
        path: &Path,
        source: &str,
    ) -> Result<(), Failure> {
        let services = Self::service_names(snapshot_root);
        let needed = RubyNeeds::typed(source)
            .into_iter()
            .find(|field| services.contains(&field.type_name));
        needed.map_or(Ok(()), |field| Err(self.failure(path, &field)))
    }

    fn service_names(snapshot_root: &Path) -> BTreeSet<String> {
        std::fs::read_dir(snapshot_root.join(Self::SERVICES))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
            .filter_map(|entry| Self::stem(&entry.path()))
            .collect()
    }

    fn stem(path: &Path) -> Option<String> {
        path.file_stem()?.to_str().map(str::to_owned)
    }

    fn failure(&self, path: &Path, field: &NeededField) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let span = source_span(&portable, field.span.start, field.span.end);
        Failure::one(diagnostic(
            Self::CODE,
            self.message(field),
            Some(self.help(field)),
            span,
        ))
    }

    fn message(&self, field: &NeededField) -> String {
        let name = &field.name;
        let service = &field.type_name;
        let language = self.language;
        let decision = self.decision;
        format!(
            "The `#[Needs]` field '{name}' names {service}, another Tac service. A {language} Tac service cannot need another service yet ({decision}): the page does not fill the field, so it would stay nil and the first call through it would fail."
        )
    }

    fn help(&self, field: &NeededField) -> String {
        let service = &field.type_name;
        let language = self.language;
        let injected = self.injected;
        let supporting = Self::SUPPORTING;
        format!(
            "A {language} Needs field takes {injected}. Let the view call {service} itself, publish an event that {service} handles in an OnEvent method, or write both services in a Tac language that can need another service: {supporting}."
        )
    }
}
