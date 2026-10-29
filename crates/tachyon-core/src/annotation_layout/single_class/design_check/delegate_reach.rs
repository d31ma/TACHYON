use super::super::duplication::ProjectSource;
use crate::delegate_forwards::DelegateForwards;
use std::path::Path;

pub(super) struct DelegateReach {
    takers: Vec<(String, String)>,
}

impl DelegateReach {
    pub(super) fn read(sources: &[ProjectSource]) -> Self {
        let forwards = DelegateForwards::new(
            sources
                .iter()
                .map(|source| (Path::new(&source.path), source.text.as_bytes())),
        );
        Self {
            takers: sources
                .iter()
                .filter_map(|source| Self::taker(&forwards, source))
                .collect(),
        }
    }

    pub(super) fn delegates_of(&self, path: &str) -> Vec<&str> {
        let path = Path::new(path);
        let service = Self::key(path, &Self::class_name(path));
        self.takers
            .iter()
            .filter(|(base, _)| *base == service)
            .map(|(_, delegate)| delegate.as_str())
            .collect()
    }

    fn taker(forwards: &DelegateForwards<'_>, source: &ProjectSource) -> Option<(String, String)> {
        let path = Path::new(&source.path);
        let base = forwards.taken_on(path, &source.text)?;
        Some((Self::key(path, &base.service), Self::class_name(path)))
    }

    fn key(path: &Path, class: &str) -> String {
        let realm = path
            .components()
            .next()
            .map(|realm| realm.as_os_str().to_string_lossy())
            .unwrap_or_default();
        let extension = path
            .extension()
            .map(|extension| extension.to_string_lossy())
            .unwrap_or_default();
        format!("{realm}/{class}.{extension}")
    }

    fn class_name(path: &Path) -> String {
        path.file_stem()
            .map(|stem| stem.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}
