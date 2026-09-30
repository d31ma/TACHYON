use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) struct RoleFiles;

impl RoleFiles {
    const REALMS: [&'static str; 2] = ["client", "server"];
    const UTILITY_FOLDER: &'static str = "utils";
    const TEST_FOLDER: &'static str = "tests";
    const SOURCE_EXTENSIONS: [&'static str; 14] = [
        "js", "ts", "py", "php", "java", "kt", "cs", "fs", "rs", "swift", "dart", "go", "rb", "lua",
    ];

    pub(crate) fn sources(project: &Path) -> Vec<(PathBuf, String)> {
        Self::paths(project)
            .into_iter()
            .filter_map(|relative| Self::read(project, relative))
            .collect()
    }

    pub(crate) fn newest_change(project: &Path) -> Option<SystemTime> {
        let mut paths = Self::paths(project);
        paths.extend(Self::test_paths(project));
        paths
            .iter()
            .filter_map(|relative| Self::modified(project, relative))
            .max()
    }

    pub(crate) fn contains(relative: &Path) -> bool {
        Self::folders()
            .iter()
            .any(|folder| relative.starts_with(folder))
    }

    fn read(project: &Path, relative: PathBuf) -> Option<(PathBuf, String)> {
        let source = std::fs::read_to_string(project.join(&relative)).ok()?;
        Some((relative, source))
    }

    fn paths(project: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for folder in Self::folders() {
            Self::collect(project, &folder, &mut found);
        }
        found.sort();
        found
    }

    fn test_paths(project: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        for realm in Self::REALMS {
            Self::collect(
                project,
                &Path::new(realm).join(Self::TEST_FOLDER),
                &mut found,
            );
        }
        found
    }

    fn modified(project: &Path, relative: &Path) -> Option<SystemTime> {
        std::fs::metadata(project.join(relative))
            .and_then(|metadata| metadata.modified())
            .ok()
    }

    fn folders() -> Vec<PathBuf> {
        let mut names = crate::annotation_modules::role_folders().to_vec();
        names.push(Self::UTILITY_FOLDER);
        Self::REALMS
            .iter()
            .flat_map(|realm| names.iter().map(move |name| Path::new(realm).join(name)))
            .collect()
    }

    fn collect(project: &Path, relative: &Path, found: &mut Vec<PathBuf>) {
        for (child, is_directory) in Self::children(project, relative) {
            Self::keep(project, child, is_directory, found);
        }
    }

    fn keep(project: &Path, child: PathBuf, is_directory: bool, found: &mut Vec<PathBuf>) {
        if is_directory {
            return Self::collect(project, &child, found);
        }
        found.extend(Some(child).filter(|path| Self::is_source(path)));
    }

    fn children(project: &Path, relative: &Path) -> Vec<(PathBuf, bool)> {
        std::fs::read_dir(project.join(relative))
            .into_iter()
            .flatten()
            .flatten()
            .map(|entry| (relative.join(entry.file_name()), entry.path().is_dir()))
            .collect()
    }

    fn is_source(path: &Path) -> bool {
        path.extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| Self::SOURCE_EXTENSIONS.contains(&extension))
    }
}
