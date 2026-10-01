use super::{Failure, Module, SharedImports, read_module_text};
use crate::compiler::script_imports::ScriptImports;
use cap_std::fs::Dir;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(Default)]
pub(super) struct ClientUtilities {
    imported: BTreeSet<PathBuf>,
    implicit: Vec<String>,
}

impl ClientUtilities {
    pub(super) fn imported(
        root: &Dir,
        entry: &Path,
        entry_source: &str,
        paths: &[PathBuf],
    ) -> Result<Self, Failure> {
        Self::collect(root, entry, entry_source, paths)
    }

    pub(super) fn contains(&self, path: &Path) -> bool {
        self.imported.contains(path)
    }

    pub(super) fn with_source(&self, source: &str) -> String {
        format!("{}{source}", self.implicit.concat())
    }

    pub(super) fn changed(&self) -> bool {
        !self.implicit.is_empty()
    }

    fn collect(
        root: &Dir,
        entry: &Path,
        entry_source: &str,
        paths: &[PathBuf],
    ) -> Result<Self, Failure> {
        Self::script_entry(entry)
            .then(|| Self::collect_script(root, entry, entry_source, paths))
            .transpose()
            .map(Option::unwrap_or_default)
    }

    fn collect_script(
        root: &Dir,
        entry: &Path,
        entry_source: &str,
        paths: &[PathBuf],
    ) -> Result<Self, Failure> {
        let mut imported = BTreeSet::new();
        Self::add(entry, entry_source, &mut imported);
        paths.iter().try_for_each(|path| {
            let source = read_module_text(root, path, Module::MAX_SOURCE)?;
            Self::add(path, &source, &mut imported);
            Ok::<_, Failure>(())
        })?;
        let mut utilities = Self {
            imported,
            implicit: Vec::new(),
        };
        paths
            .iter()
            .try_for_each(|path| utilities.preserve_module(root, entry, path))?;
        Ok(utilities)
    }

    fn preserve_module(&mut self, root: &Dir, entry: &Path, path: &Path) -> Result<(), Failure> {
        let candidate = path.starts_with("client/utils") && !self.imported.contains(path);
        candidate
            .then(|| self.project_module(root, entry, path))
            .transpose()
            .map(|_| ())
    }

    fn project_module(&mut self, root: &Dir, entry: &Path, path: &Path) -> Result<(), Failure> {
        let source = read_module_text(root, path, Module::MAX_SOURCE)?;
        self.add_module(entry, path, &source)
    }

    fn add_module(&mut self, entry: &Path, path: &Path, source: &str) -> Result<(), Failure> {
        if !crate::compiler::utility_module::UtilityModule::needs_module(path, source)? {
            return Ok(());
        }
        crate::annotation_layout::UtilityLayout::check(path, source)?;
        let name = path
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("");
        let relative = SharedImports::relative_path(entry, path);
        self.implicit
            .push(format!("import {{ {name} }} from '{relative}';\n"));
        self.imported.insert(path.to_path_buf());
        Ok(())
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
