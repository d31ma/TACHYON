use crate::Failure;
use crate::annotation_layout::UtilityLayout;
use crate::method_coverage::CallHooks;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(super) struct HookedUtilities;

impl HookedUtilities {
    const IMPORT_PREFIX: &'static str = "/shared/utils/";
    const SCRIPT_EXTENSIONS: [&'static str; 2] = ["js", "ts"];
    const RECORDER_PRELUDE: &'static str = "const Behaviors = { called: (key) => globalThis[Symbol.for('tachyon.unit.behaviors')]?.called(key) };\n";
    pub(super) const RECORDER_PUBLISH: &'static str =
        "globalThis[Symbol.for('tachyon.unit.behaviors')] = Behaviors;\n";

    pub(super) fn import_mapping(stage: &Path) -> std::io::Result<Option<(String, String)>> {
        let copies = stage.join(UtilityLayout::CLIENT_ROOT);
        if !copies.is_dir() {
            return Ok(None);
        }
        let target = super::ModuleUrl::directory(&copies)?;
        Ok(Some((String::from(Self::IMPORT_PREFIX), target)))
    }

    pub(super) fn write(project: &Path, stage: &Path) -> Result<(), Failure> {
        Self::scripts(project, Path::new(UtilityLayout::CLIENT_ROOT))
            .iter()
            .try_for_each(|relative| Self::write_copy(project, stage, relative))
    }

    fn write_copy(project: &Path, stage: &Path, relative: &Path) -> Result<(), Failure> {
        let portable = relative.to_string_lossy();
        let source = std::fs::read_to_string(project.join(relative))
            .map_err(|error| super::stage_failure(&portable, &error))?;
        let hooked = CallHooks::insert(relative, &source)?;
        let copy = stage.join(relative);
        copy.parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|()| std::fs::write(&copy, format!("{}{hooked}", Self::RECORDER_PRELUDE)))
            .map_err(|error| super::stage_failure(&portable, &error))
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
