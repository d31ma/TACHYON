use super::python_provider_source::PythonProviderSource;
use crate::Failure;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::path::Path;

pub(super) struct PythonGraphGuard;

impl PythonGraphGuard {
    pub(super) fn check(providers: &Map<String, Value>) -> Result<(), Failure> {
        providers
            .keys()
            .try_for_each(|name| Self::visit(name, providers, &BTreeSet::new()).map(|_| ()))
    }

    fn visit(
        name: &str,
        providers: &Map<String, Value>,
        ancestors: &BTreeSet<String>,
    ) -> Result<usize, Failure> {
        Self::require_acyclic(name, ancestors)?;
        Self::count(name, providers, ancestors)
    }

    fn count(
        name: &str,
        providers: &Map<String, Value>,
        ancestors: &BTreeSet<String>,
    ) -> Result<usize, Failure> {
        let mut next = ancestors.clone();
        next.insert(name.to_owned());
        Self::children(name, providers).try_fold(1_usize, |total, child| {
            Self::visit(child, providers, &next).and_then(|count| Self::bounded_count(total, count))
        })
    }

    fn bounded_count(total: usize, count: usize) -> Result<usize, Failure> {
        let count = total.saturating_add(count);
        Self::require_count(count)?;
        Ok(count)
    }

    fn require_count(count: usize) -> Result<(), Failure> {
        if count > 4096 {
            return Err(PythonProviderSource::failure(
                Path::new("client/services"),
                "Python Tac dependency graph exceeds 4096 provider acquisitions.",
            ));
        }
        Ok(())
    }

    fn require_acyclic(name: &str, ancestors: &BTreeSet<String>) -> Result<(), Failure> {
        if ancestors.contains(name) || ancestors.len() >= 32 {
            return Err(PythonProviderSource::failure(
                Path::new("client/services"),
                "Python Tac dependency graph contains a cycle or exceeds 32 levels.",
            ));
        }
        Ok(())
    }

    fn children<'a>(
        name: &str,
        providers: &'a Map<String, Value>,
    ) -> impl Iterator<Item = &'a str> {
        providers[name]["constructor"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|binding| binding["provider"].as_str())
    }
}
