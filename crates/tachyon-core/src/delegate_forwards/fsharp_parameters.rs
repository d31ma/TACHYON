use super::{DelegateForwards, ParameterSplit};
use crate::annotation_rules::NeededField;
use std::path::Path;

pub(super) struct FsharpParameters;

impl FsharpParameters {
    const EXTENSION: &'static str = "fs";
    const TYPE_WORD: &'static str = "type ";

    pub(super) fn needed(path: &Path, source: &str) -> Vec<NeededField> {
        let fsharp = path.extension().is_some_and(|extension| extension == Self::EXTENSION);
        let masked = fsharp
            .then(|| crate::annotation_layout::masked_source(path, source).ok())
            .flatten()
            .unwrap_or_default();
        let header = format!("{}{}", Self::TYPE_WORD, DelegateForwards::class_name(path));
        masked
            .find(&header)
            .and_then(|at| {
                let open = at + header.len();
                masked[open..].starts_with('(').then_some(open)
            })
            .and_then(|open| Some((open, ParameterSplit::matching_close(&masked, open)?)))
            .map(|(open, close)| Self::fields(source, &masked, open + 1, close))
            .unwrap_or_default()
    }

    fn fields(source: &str, masked: &str, start: usize, close: usize) -> Vec<NeededField> {
        ParameterSplit::parts(source, masked, start..close)
            .into_iter()
            .filter_map(|part| {
                let name = part.split(':').next()?.trim().to_owned();
                let at = start + source[start..close].find(&name)?;
                Some(NeededField { name, at })
            })
            .collect()
    }
}
