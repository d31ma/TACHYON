use crate::Failure;
use crate::failure::{diagnostic, source_span};
use crate::ruby_source::{NeededField, RubyNeeds, UntypedField};
use std::collections::BTreeSet;
use std::ops::Range;
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
        injected: "a Store, a Converter, a Helper, an HTTPClient or a browser API such as Clock or Clipboard",
    };
    pub(crate) const LUA: Self = Self {
        language: "Lua",
        decision: "ADR 0085",
        injected: "a Store, a Converter, a Helper or an HTTPClient",
    };
    const CODE: u16 = 2015;
    const SERVICES: &'static str = "client/services";
    const SUPPORTING: &'static str = "JavaScript, TypeScript, Go, Python, Dart, Java, C++ or F#";

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
        needed.map_or(Ok(()), |field| {
            Err(Self::failure(
                path,
                &field.span,
                self.message(&field),
                self.help(&field),
            ))
        })
    }

    pub(crate) fn refuse_untyped(&self, path: &Path, source: &str) -> Result<(), Failure> {
        let untyped = RubyNeeds::untyped(source).into_iter().next();
        untyped.map_or(Ok(()), |field| {
            Err(Self::failure(
                path,
                &field.span,
                self.untyped_message(&field),
                self.untyped_help(),
            ))
        })
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

    fn failure(path: &Path, span: &Range<usize>, message: String, help: String) -> Failure {
        let portable = path.to_string_lossy().replace('\\', "/");
        let located = source_span(&portable, span.start, span.end);
        Failure::one(diagnostic(Self::CODE, message, Some(help), located))
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

    fn untyped_message(&self, field: &UntypedField) -> String {
        let name = &field.name;
        let language = self.language;
        let decision = self.decision;
        format!(
            "The `#[Needs]` field '{name}' has no type. A {language} Tac service fills a Needs field from the `# @return [Type]` comment next to it ({decision}): without one the page does not fill the field, so it would stay nil and the first call through it would fail."
        )
    }

    fn untyped_help(&self) -> String {
        let language = self.language;
        let injected = self.injected;
        format!(
            "Write the type in a comment next to the field, such as `# @return [CounterStore]` or `# @return [Clipboard]`. A {language} Needs field takes {injected}."
        )
    }
}
