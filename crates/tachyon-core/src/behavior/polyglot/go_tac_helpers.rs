use super::super::Input;
use crate::Failure;
use std::path::Path;

pub(super) struct GoTacHelpers;

impl GoTacHelpers {
    const LANGUAGE: &'static str = "go";
    const MANAGED: [&'static str; 3] = ["Needs", "NeedsAll", "Lifetime"];
    const ADVICE: &'static str = "A Go Tac Delegate belongs to the service that needs or creates it, so the Delegate has no Needs, NeedsAll or Lifetime of its own. Give the Delegate what it uses through method parameters, or let the service set its fields after creating it.";

    pub(super) fn source(path: &Path, source: &str) -> Result<(), Failure> {
        Self::client_path(path)
            .then(|| Self::managed_markers(path, source))
            .transpose()
            .map(|_| ())
    }

    fn managed_markers(path: &Path, source: &str) -> Result<(), Failure> {
        let mask = crate::lexical::code_mask(Self::LANGUAGE, source);
        let input = Input {
            path,
            source,
            code: super::visible_code(source, mask.code),
        };
        let markers = super::annotations(&input, Self::LANGUAGE, "tac")?;
        let helper = markers.iter().any(|marker| marker.name == "Delegate");
        markers
            .iter()
            .find(|marker| Self::MANAGED.contains(&marker.name.as_str()))
            .filter(|_| helper)
            .map_or(Ok(()), |marker| {
                Err(input.failure(marker.span.start, Self::ADVICE))
            })
    }

    fn client_path(path: &Path) -> bool {
        let extension = path.extension().and_then(|value| value.to_str());
        let realm = crate::annotation_rules::Realm::of(path);
        (extension, realm)
            == (
                Some(Self::LANGUAGE),
                Some(crate::annotation_rules::Realm::Tac),
            )
    }
}
