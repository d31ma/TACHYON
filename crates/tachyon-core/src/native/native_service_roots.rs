use crate::template::{RequiredFields, ServiceDefinition, ServiceRegistry};
use std::collections::BTreeSet;

pub(super) struct NativeServiceRoots;

impl NativeServiceRoots {
    pub(super) fn expand(registry: &ServiceRegistry, roots: &BTreeSet<String>) -> BTreeSet<String> {
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
            .and_then(|path| {
                std::fs::read_to_string(path).ok().map(|source| {
                    RequiredFields::service_dependencies(path, &source, definition.class(), names)
                })
            })
            .unwrap_or_default()
    }
}
