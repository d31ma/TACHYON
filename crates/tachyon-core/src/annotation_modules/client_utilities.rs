use super::{Failure, Module, SharedImports, read_module_text};
use crate::compiler::script_imports::ScriptImports;
use cap_std::fs::Dir;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

pub(super) struct ClientUtilities;

impl ClientUtilities {
    pub(super) fn imported(
        root: &Dir,
        entry: &Path,
        entry_source: &str,
        paths: &[PathBuf],
    ) -> Result<BTreeSet<PathBuf>, Failure> {
        Self::collect(root, entry, entry_source, paths)
    }

    fn collect(
        root: &Dir,
        entry: &Path,
        entry_source: &str,
        paths: &[PathBuf],
    ) -> Result<BTreeSet<PathBuf>, Failure> {
        let mut imported = BTreeSet::new();
        if !Self::script_entry(entry) {
            return Ok(imported);
        }
        Self::add(entry, entry_source, &mut imported);
        paths.iter().try_for_each(|path| {
            let source = read_module_text(root, path, Module::MAX_SOURCE)?;
            Self::add(path, &source, &mut imported);
            Ok::<_, Failure>(())
        })?;
        Ok(imported)
    }

    fn script_entry(entry: &Path) -> bool {
        let extension = entry
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or("");
        [
            entry.starts_with("client"),
            ["js", "ts"].contains(&extension),
        ]
        .into_iter()
        .all(|holds| holds)
    }

    fn add(path: &Path, source: &str, imported: &mut BTreeSet<PathBuf>) {
        ScriptImports::read(source)
            .into_iter()
            .filter_map(|import| Self::target(path, &import.specifier))
            .filter(|target| target.starts_with("client/utils"))
            .for_each(|target| {
                target
                    .extension()
                    .is_some_and(|extension| extension == "js")
                    .then(|| imported.insert(target.with_extension("ts")));
                imported.insert(target);
            });
    }

    fn target(path: &Path, specifier: &str) -> Option<PathBuf> {
        let specifier = specifier.split(['?', '#']).next()?;
        specifier
            .strip_prefix("/utils/")
            .map(|tail| PathBuf::from("client/utils").join(tail))
            .or_else(|| SharedImports::path(path, specifier))
    }
}
