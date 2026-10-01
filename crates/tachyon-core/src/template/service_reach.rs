use super::script_service_references::ScriptServiceReferences;
use super::{RequiredFields, ServiceDefinition, ServiceRegistry};
use std::collections::BTreeSet;

pub(crate) struct ServiceReach;

impl ServiceReach {
    pub(crate) fn expand(registry: &ServiceRegistry, roots: &BTreeSet<String>) -> BTreeSet<String> {
        let names = registry.names();
        Self::traverse(registry, roots, &names)
    }

    fn traverse(
        registry: &ServiceRegistry,
        roots: &BTreeSet<String>,
        names: &BTreeSet<String>,
    ) -> BTreeSet<String> {
        let mut reached = BTreeSet::new();
        let mut pending = roots.clone();
        while let Some(name) = pending.pop_first() {
            Self::visit(&name, (registry, names), &mut reached, &mut pending);
        }
        reached
    }

    fn visit(
        name: &str,
        definitions: (&ServiceRegistry, &BTreeSet<String>),
        reached: &mut BTreeSet<String>,
        pending: &mut BTreeSet<String>,
    ) {
        if !reached.insert(name.to_owned()) {
            return;
        }
        let dependencies = definitions
            .0
            .get(name)
            .filter(|definition| !definition.linked())
            .map(|definition| Self::dependencies(definition, definitions.1))
            .unwrap_or_default();
        pending.extend(dependencies);
    }

    fn dependencies(definition: &ServiceDefinition, names: &BTreeSet<String>) -> BTreeSet<String> {
        definition
            .script_path()
            .map(|path| Self::source_dependencies(definition, names, path))
            .unwrap_or_default()
    }

    fn source_dependencies(
        definition: &ServiceDefinition,
        names: &BTreeSet<String>,
        path: &std::path::Path,
    ) -> BTreeSet<String> {
        std::fs::read_to_string(path).map_or_else(
            |_| names.clone(),
            |source| Self::declared_dependencies(definition, names, path, &source),
        )
    }

    fn declared_dependencies(
        definition: &ServiceDefinition,
        names: &BTreeSet<String>,
        path: &std::path::Path,
        source: &str,
    ) -> BTreeSet<String> {
        match path.extension().and_then(std::ffi::OsStr::to_str) {
            Some("js" | "ts") => ScriptServiceReferences::read(path, source, names),
            _ => RequiredFields::service_dependencies(path, source, definition.class(), names),
        }
    }
}
