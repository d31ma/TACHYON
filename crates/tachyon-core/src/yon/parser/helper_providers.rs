use super::{Input, ManagedType, constructor_probe};
use crate::Failure;

pub(super) struct HelperProviders;

impl HelperProviders {
    pub(super) fn select(
        input: &Input<'_>,
        types: Vec<ManagedType>,
    ) -> Result<Vec<ManagedType>, Failure> {
        let selected: Result<Vec<_>, _> = types
            .into_iter()
            .map(|managed| Self::selected(input, managed))
            .collect();
        Ok(selected?.into_iter().flatten().collect())
    }

    fn selected(input: &Input<'_>, managed: ManagedType) -> Result<Option<ManagedType>, Failure> {
        let manual = Self::manually_constructed(input, &managed)?;
        Ok((!manual).then_some(managed))
    }

    fn manually_constructed(input: &Input<'_>, managed: &ManagedType) -> Result<bool, Failure> {
        input
            .manual_helpers
            .contains(&managed.provider.name)
            .then(|| Self::manual_constructor(input, managed))
            .transpose()
            .map(|result| result.unwrap_or(false))
    }

    pub(super) fn count(input: &Input<'_>, managed: &ManagedType) -> Result<usize, Failure> {
        let count = constructor_probe(input, managed)?;
        Self::required_constructor(input, managed, count)
    }

    fn required_constructor(
        input: &Input<'_>,
        managed: &ManagedType,
        count: Option<usize>,
    ) -> Result<usize, Failure> {
        match (input.extension, count) {
            ("rs", None) => Err(input.failure(
                managed.declaration,
                format!(
                    "Managed Rust providers require an inherent fn new(...) constructor for {}",
                    managed.provider.name
                ),
            )),
            (_, count) => Ok(count.unwrap_or(0)),
        }
    }

    fn manual_constructor(input: &Input<'_>, managed: &ManagedType) -> Result<bool, Failure> {
        let count = constructor_probe(input, managed)?;
        Ok(Self::manual_count(input.extension, count))
    }

    fn manual_count(extension: &str, count: Option<usize>) -> bool {
        match (extension, count) {
            ("go", _) | ("rs", None) => true,
            (_, count) => count.unwrap_or(0) > 0,
        }
    }
}
