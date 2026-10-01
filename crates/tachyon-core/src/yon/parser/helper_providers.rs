use super::{Input, ManagedType, constructor_count};
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

    fn manual_constructor(input: &Input<'_>, managed: &ManagedType) -> Result<bool, Failure> {
        match input.extension {
            "go" => Ok(true),
            _ => constructor_count(input, managed).map(|count| count > 0),
        }
    }
}
