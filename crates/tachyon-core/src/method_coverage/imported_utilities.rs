use crate::annotation_layout::UtilityLayout;
use crate::compiler::script_imports::ScriptImports;
use crate::compiler::utility_module::UtilityModule;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

pub(crate) struct ImportedUtilities;

impl ImportedUtilities {
    const CLIENT: &'static str = "client";
    const SCRIPT_EXTENSIONS: [&'static str; 2] = ["js", "ts"];
    const TYPESCRIPT: &'static str = "ts";

    pub(crate) fn unrecorded(project: &Path) -> BTreeSet<PathBuf> {
        Self::scripts(project, Path::new(Self::CLIENT))
            .iter()
            .flat_map(|path| Self::unrecorded_from(project, path))
            .collect()
    }

    fn unrecorded_from(project: &Path, path: &Path) -> Vec<PathBuf> {
        let source = std::fs::read_to_string(project.join(path)).unwrap_or_default();
        let mut found: Vec<PathBuf> = ScriptImports::read(&source)
            .iter()
            .filter_map(|import| Self::target(path, &import.specifier))
            .flat_map(Self::spellings)
            .filter(|target| UtilityLayout::is_client_utility(target))
            .collect();
        found.extend(Self::module_utility(path, &source));
        found
    }

    fn module_utility(path: &Path, source: &str) -> Option<PathBuf> {
        let module = UtilityModule::needs_module(path, source).unwrap_or(false);
        (module && UtilityLayout::is_client_utility(path)).then(|| path.to_path_buf())
    }

    fn spellings(target: PathBuf) -> [PathBuf; 2] {
        let typescript = target.with_extension(Self::TYPESCRIPT);
        [target, typescript]
    }

    fn target(path: &Path, specifier: &str) -> Option<PathBuf> {
        let specifier = specifier.split(['?', '#']).next()?;
        specifier
            .strip_prefix('/')
            .map(|rest| Path::new(Self::CLIENT).join(rest))
            .or_else(|| Self::relative(path, specifier))
    }

    fn relative(path: &Path, specifier: &str) -> Option<PathBuf> {
        specifier.starts_with('.').then_some(())?;
        let mut parts: Vec<&OsStr> = path.parent()?.iter().collect();
        for part in Path::new(specifier).components() {
            Self::step(&mut parts, part)?;
        }
        Some(parts.iter().collect())
    }

    fn step<'a>(parts: &mut Vec<&'a OsStr>, part: Component<'a>) -> Option<()> {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::Normal(name) => parts.push(name),
            Component::RootDir | Component::Prefix(_) => return None,
        }
        Some(())
    }

    fn scripts(project: &Path, relative: &Path) -> Vec<PathBuf> {
        std::fs::read_dir(project.join(relative))
            .into_iter()
            .flatten()
            .flatten()
            .flat_map(|entry| {
                Self::entry_scripts(
                    project,
                    relative.join(entry.file_name()),
                    entry.path().is_dir(),
                )
            })
            .collect()
    }

    fn entry_scripts(project: &Path, child: PathBuf, is_directory: bool) -> Vec<PathBuf> {
        if is_directory {
            return Self::scripts(project, &child);
        }
        Some(child)
            .filter(|path| Self::is_script(path))
            .into_iter()
            .collect()
    }

    fn is_script(path: &Path) -> bool {
        path.extension()
            .and_then(OsStr::to_str)
            .is_some_and(|extension| Self::SCRIPT_EXTENSIONS.contains(&extension))
    }
}
