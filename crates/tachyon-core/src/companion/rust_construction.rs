use crate::Failure;
use crate::failure::{diagnostic, source_span};
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct RustConstruction {
    source: String,
    path: String,
    fields: BTreeMap<String, Vec<(String, String)>>,
    constructors: BTreeMap<String, Vec<String>>,
    defaults: BTreeSet<String>,
    arc_cache: RefCell<BTreeMap<String, String>>,
    bindings: RefCell<Vec<String>>,
}

impl RustConstruction {
    const MAX_DEPTH: usize = 32;
    const BUILTIN_PROVIDERS: [&'static str; 2] = ["FilePicker", "WindowControls"];

    pub(super) fn instance(
        source: &str,
        path: &str,
        name: &str,
        needs_all: bool,
    ) -> Result<String, Failure> {
        let mut construction = Self {
            source: crate::annotation_layout::masked_source(std::path::Path::new(path), source)?,
            path: path.to_owned(),
            fields: BTreeMap::new(),
            constructors: BTreeMap::new(),
            defaults: BTreeSet::new(),
            arc_cache: RefCell::new(BTreeMap::new()),
            bindings: RefCell::new(Vec::new()),
        };
        construction.read_types();
        construction.read_constructors();
        construction.read_defaults();
        let value = construction.construct(name, needs_all, &[])?;
        Ok(construction.bound_expression(&value))
    }

    fn read_types(&mut self) {
        for (header, body) in super::blocks(&self.source, "struct") {
            let name = super::identifier(header.trim());
            let fields = body
                .lines()
                .flat_map(super::rust_field_declarations)
                .filter_map(|declaration| Self::field_declaration(&declaration))
                .collect();
            self.fields.insert(name, fields);
        }
    }

    fn field_declaration(declaration: &str) -> Option<(String, String)> {
        let declaration = declaration
            .trim()
            .trim_start_matches("pub ")
            .trim_start_matches("pub(crate) ");
        let name = super::identifier(declaration);
        let kind = declaration[name.len()..]
            .trim_start()
            .strip_prefix(':')?
            .trim()
            .trim_end_matches(',')
            .trim();
        Some((name, kind.to_owned()))
    }

    fn bound_expression(&self, value: &str) -> String {
        let bindings = self.bindings.borrow().join("\n");
        if bindings.is_empty() {
            return value.to_owned();
        }
        format!("{{ {bindings}\n{value} }}")
    }

    fn read_constructors(&mut self) {
        let constructors = super::blocks(&self.source, "impl")
            .into_iter()
            .filter_map(|(header, body)| {
                Self::constructor_parameters(body)
                    .map(|parameters| (header.trim().to_owned(), parameters))
            })
            .collect::<BTreeMap<_, _>>();
        self.constructors.extend(constructors);
    }

    fn constructor_parameters(body: &str) -> Option<Vec<String>> {
        let start = body.find("fn new")?;
        let after_name = body[start + "fn new".len()..].trim_start();
        let rest = after_name.strip_prefix('(')?;
        let close = super::closing_parenthesis(after_name)?;
        let result = after_name[close + 1..]
            .trim_start()
            .strip_prefix("->")?
            .trim_start();
        (super::identifier(result) == "Self").then_some(())?;
        let arguments = &rest[..close - 1];
        Some(
            arguments
                .split(',')
                .filter(|argument| !argument.trim().is_empty())
                .map(Self::parameter_type)
                .collect(),
        )
    }

    fn parameter_type(argument: &str) -> String {
        argument
            .split_once(':')
            .map_or("", |(_, kind)| kind)
            .trim()
            .to_owned()
    }

    fn read_defaults(&mut self) {
        self.defaults.extend(
            super::blocks(&self.source, "impl")
                .into_iter()
                .filter_map(Self::default_implementation),
        );
        let mut previous = String::new();
        for line in self.source.lines() {
            Self::remember_derive(&mut self.defaults, &mut previous, line);
        }
    }

    fn default_implementation((header, _): (&str, &str)) -> Option<String> {
        let (kind, name) = header.trim().split_once(" for ")?;
        let available = matches!(
            kind,
            "Default"
                | "core::default::Default"
                | "::core::default::Default"
                | "std::default::Default"
                | "::std::default::Default"
        );
        available.then(|| name.trim().to_owned())
    }

    fn remember_derive(defaults: &mut BTreeSet<String>, previous: &mut String, line: &str) {
        let trimmed = line.trim();
        Self::remember_derived_type(defaults, previous, trimmed);
        Self::collect_attribute(previous, trimmed);
    }

    fn collect_attribute(previous: &mut String, line: &str) {
        if line.starts_with('#') || line.is_empty() {
            previous.push_str(line);
        } else {
            previous.clear();
        }
    }

    fn remember_derived_type(defaults: &mut BTreeSet<String>, attributes: &str, declaration: &str) {
        let derived = attributes.contains("derive(");
        let default = attributes
            .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
            .any(|word| word == "Default");
        let header = declaration
            .trim_start_matches("pub ")
            .trim_start_matches("pub(crate) ");
        let name = header.strip_prefix("struct ").map(super::identifier);
        let name = (derived & default).then_some(name).flatten();
        let _ = name.map(|name| defaults.insert(name));
    }

    fn construct(
        &self,
        name: &str,
        fills_fields: bool,
        trail: &[String],
    ) -> Result<String, Failure> {
        if trail.len() >= Self::MAX_DEPTH || trail.iter().any(|ancestor| ancestor == name) {
            return Err(self.failure(
                name,
                "contains a dependency cycle or exceeds 32 provider levels",
            ));
        }
        let mut descendants = trail.to_vec();
        descendants.push(name.to_owned());
        self.initializer(name, fills_fields, &descendants)
    }

    fn initializer(
        &self,
        name: &str,
        fills_fields: bool,
        trail: &[String],
    ) -> Result<String, Failure> {
        self.constructors
            .get(name)
            .map(|parameters| self.constructor(name, parameters, trail))
            .or_else(|| self.default(name))
            .or_else(|| fills_fields.then(|| self.filled(name, trail)))
            .unwrap_or_else(|| Err(super::missing_constructor(&self.path, name)))
    }

    fn default(&self, name: &str) -> Option<Result<String, Failure>> {
        let available = self.defaults.contains(name) || Self::BUILTIN_PROVIDERS.contains(&name);
        available.then(|| Ok(format!("<{name} as ::core::default::Default>::default()")))
    }

    fn constructor(
        &self,
        name: &str,
        parameters: &[String],
        trail: &[String],
    ) -> Result<String, Failure> {
        let arguments = parameters
            .iter()
            .map(|kind| self.provider(kind, trail))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("{name}::new({})", arguments.join(", ")))
    }

    fn provider(&self, kind: &str, trail: &[String]) -> Result<String, Failure> {
        Self::arc_inner(kind)
            .map_or_else(|| self.nominal(kind, trail), |inner| self.arc(inner, trail))
    }

    fn nominal(&self, kind: &str, trail: &[String]) -> Result<String, Failure> {
        self.require_provider(kind)?;
        self.construct(kind, false, trail)
    }

    fn arc_inner(kind: &str) -> Option<&str> {
        let kind = kind.trim();
        let (wrapper, parameters) = kind.split_once('<')?;
        let inner = parameters.strip_suffix('>')?.trim();
        matches!(
            wrapper.trim(),
            "Arc" | "std::sync::Arc" | "::std::sync::Arc"
        )
        .then_some(inner)
    }

    fn arc(&self, kind: &str, trail: &[String]) -> Result<String, Failure> {
        let cached = self.arc_cache.borrow().get(kind).cloned();
        cached.map_or_else(
            || self.new_arc(kind, trail),
            |binding| Ok(format!("::std::sync::Arc::clone(&{binding})")),
        )
    }

    fn new_arc(&self, kind: &str, trail: &[String]) -> Result<String, Failure> {
        let value = self.nominal(kind, trail)?;
        let binding = format!("tac_provider_{}", self.bindings.borrow().len());
        self.bindings
            .borrow_mut()
            .push(format!("let {binding} = ::std::sync::Arc::new({value});"));
        self.arc_cache
            .borrow_mut()
            .insert(kind.to_owned(), binding.clone());
        Ok(format!("::std::sync::Arc::clone(&{binding})"))
    }

    fn require_provider(&self, kind: &str) -> Result<(), Failure> {
        if !self.fields.contains_key(kind) && !Self::BUILTIN_PROVIDERS.contains(&kind) {
            return Err(self.failure(
                kind,
                "has no same-language declared provider for its required constructor input",
            ));
        }
        Ok(())
    }

    fn filled(&self, name: &str, trail: &[String]) -> Result<String, Failure> {
        let fields = self
            .fields
            .get(name)
            .ok_or_else(|| super::missing_constructor(&self.path, name))?;
        let arguments = fields
            .iter()
            .map(|(field, kind)| self.field(field, kind, trail))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(format!("{name} {{ {} }}", arguments.join(", ")))
    }

    fn field(&self, name: &str, kind: &str, trail: &[String]) -> Result<String, Failure> {
        let value = Self::arc_inner(kind)
            .map(|inner| self.arc(inner, trail))
            .or_else(|| {
                self.fields
                    .get(kind)
                    .map(|_| self.construct(kind, false, trail))
            })
            .unwrap_or_else(|| self.default_field(kind))?;
        Ok(format!("{name}: {value}"))
    }

    fn default_field(&self, kind: &str) -> Result<String, Failure> {
        let declaration = format!("field: {kind}");
        if super::rust_field(&declaration).is_none() && !Self::BUILTIN_PROVIDERS.contains(&kind) {
            return Err(self.failure(kind, "is an unsupported native constructor input; trait bindings, mutexes and owned threads need an explicit composition contract"));
        }
        Ok(String::from("::core::default::Default::default()"))
    }

    fn failure(&self, name: &str, reason: &str) -> Failure {
        Failure::one(diagnostic(
            1406,
            format!("Native Rust provider '{name}' {reason}."),
            Some(String::from(
                "Use a same-source typed provider with fn new() -> Self, typed provider constructor arguments, or a type-specific Default implementation.",
            )),
            source_span(&self.path, 0, self.path.len()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::RustConstruction;

    #[test]
    fn rust_provider_construction_reports_a_typed_dependency_cycle() {
        let source = "struct RootService { peer: PeerService }\nimpl RootService { fn new(peer: PeerService) -> Self { Self { peer } } }\nstruct PeerService { root: RootService }\nimpl PeerService { fn new(root: RootService) -> Self { Self { root } } }\n";
        let failure = RustConstruction::instance(
            source,
            "client/services/RootService.rs",
            "RootService",
            false,
        )
        .unwrap_err();
        assert!(
            failure.to_string().contains("dependency cycle"),
            "{failure}"
        );
    }

    #[test]
    fn rust_provider_construction_requires_real_declared_constructor_providers() {
        let source = "struct RootService { count: i64 }\nimpl RootService { fn new(count: i64) -> Self { Self { count } } }\n";
        let failure = RustConstruction::instance(
            source,
            "client/services/RootService.rs",
            "RootService",
            false,
        )
        .unwrap_err();
        assert!(
            failure
                .to_string()
                .contains("no same-language declared provider"),
            "{failure}"
        );
    }

    #[test]
    fn rust_provider_construction_does_not_borrow_another_types_default() {
        let source = "struct RootService { peer: PeerService }\n#[derive(Default)]\nstruct PeerService { count: i64 }\n";
        let failure = RustConstruction::instance(
            source,
            "client/services/RootService.rs",
            "RootService",
            false,
        )
        .unwrap_err();
        assert!(failure.to_string().contains("RootService"), "{failure}");
        assert!(
            failure.to_string().contains("needs a constructor"),
            "{failure}"
        );
    }

    #[test]
    fn rust_provider_construction_accepts_qualified_default_implementations() {
        for kind in [
            "Default",
            "core::default::Default",
            "::core::default::Default",
            "std::default::Default",
            "::std::default::Default",
        ] {
            let source = format!(
                "struct RootService {{ count: i64 }}\nimpl {kind} for RootService {{ fn default() -> Self {{ Self {{ count: 0 }} }} }}\n"
            );
            let expression = RustConstruction::instance(
                &source,
                "client/services/RootService.rs",
                "RootService",
                false,
            )
            .unwrap_or_else(|failure| panic!("{failure}"));
            assert_eq!(
                expression,
                "<RootService as ::core::default::Default>::default()"
            );
        }
    }

    #[test]
    fn rust_provider_construction_recursively_resolves_typed_nominal_providers() {
        let source = "struct RootService { peer: PeerService }\nimpl RootService { fn new(peer: PeerService) -> Self { Self { peer } } }\nstruct PeerService { step: StepHelper }\nimpl PeerService { fn new(step: StepHelper) -> Self { Self { step } } }\n#[derive(Default)]\nstruct StepHelper { count: i64 }\n";
        let expression = RustConstruction::instance(
            source,
            "client/services/RootService.rs",
            "RootService",
            false,
        )
        .unwrap_or_else(|failure| panic!("{failure}"));
        assert_eq!(
            expression,
            "RootService::new(PeerService::new(<StepHelper as ::core::default::Default>::default()))"
        );
    }
}
