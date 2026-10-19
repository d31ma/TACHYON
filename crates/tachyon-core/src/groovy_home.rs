use std::ffi::OsStr;
use std::path::{Path, PathBuf};

pub(crate) struct GroovyHome {
    library: PathBuf,
}

impl GroovyHome {
    const HOME_VARIABLE: &'static str = "GROOVY_HOME";
    const PATH_VARIABLE: &'static str = "PATH";
    const LAUNCHER: &'static str = "groovyc";
    const WINDOWS_LAUNCHER: &'static str = "groovyc.bat";
    const LIBRARY_FOLDER: &'static str = "lib";
    const RUNTIME_PREFIX: &'static str = "groovy-";
    const JAR_SUFFIX: &'static str = ".jar";
    const EVERY_JAR: &'static str = "*";

    pub(crate) fn find() -> Option<Self> {
        Self::from_variable().or_else(Self::from_launcher)
    }

    pub(crate) fn runtime_jar(&self) -> Option<PathBuf> {
        let mut found = self.runtime_jars();
        found.sort();
        found.pop()
    }

    pub(crate) fn class_path(&self) -> String {
        self.library
            .join(Self::EVERY_JAR)
            .to_string_lossy()
            .into_owned()
    }

    fn from_variable() -> Option<Self> {
        let home = std::env::var_os(Self::HOME_VARIABLE)?;
        Self::at(Path::new(&home))
    }

    fn from_launcher() -> Option<Self> {
        let path = std::env::var_os(Self::PATH_VARIABLE)?;
        let launcher = Self::first_file(Self::launcher_candidates(&path))?;
        let launcher = Self::resolved(&launcher).ok()?;
        let home = launcher.parent()?.parent()?;
        Self::at(home)
    }

    /// A launcher reached through a link resolves to its installed folder. Windows resolves to a
    /// `\\?\` path, which Java cannot read in a class path, and links are rare there, so on
    /// Windows the launcher path is only made absolute.
    fn resolved(launcher: &Path) -> std::io::Result<PathBuf> {
        let resolve: [fn(&Path) -> std::io::Result<PathBuf>; 2] = [
            |path| std::fs::canonicalize(path),
            |path| std::path::absolute(path),
        ];
        resolve[usize::from(cfg!(windows))](launcher)
    }

    fn at(home: &Path) -> Option<Self> {
        let library = home.join(Self::LIBRARY_FOLDER);
        if !library.is_dir() {
            return None;
        }
        Some(Self { library })
    }

    fn launcher_candidates(path: &OsStr) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        for folder in std::env::split_paths(path) {
            candidates.push(folder.join(Self::LAUNCHER));
            candidates.push(folder.join(Self::WINDOWS_LAUNCHER));
        }
        candidates
    }

    fn first_file(candidates: Vec<PathBuf>) -> Option<PathBuf> {
        candidates.into_iter().find(|candidate| candidate.is_file())
    }

    fn runtime_jars(&self) -> Vec<PathBuf> {
        let mut found = Vec::new();
        let entries = std::fs::read_dir(&self.library).into_iter().flatten();
        for entry in entries.flatten() {
            found.extend(Self::runtime_jar_path(&entry));
        }
        found
    }

    fn runtime_jar_path(entry: &std::fs::DirEntry) -> Option<PathBuf> {
        let name = entry.file_name().to_string_lossy().into_owned();
        Self::is_runtime_jar(&name).then(|| entry.path())
    }

    fn is_runtime_jar(name: &str) -> bool {
        let Some(version) = name
            .strip_prefix(Self::RUNTIME_PREFIX)
            .and_then(|rest| rest.strip_suffix(Self::JAR_SUFFIX))
        else {
            return false;
        };
        version.starts_with(|character: char| character.is_ascii_digit())
    }
}

#[cfg(test)]
mod tests {
    use super::GroovyHome;

    #[test]
    fn the_runtime_jar_is_the_core_jar_and_not_a_module_jar() -> std::io::Result<()> {
        let home = tempfile::tempdir()?;
        let library = home.path().join("lib");
        std::fs::create_dir_all(&library)?;
        for name in [
            "groovy-json-5.1.3.jar",
            "groovy-5.1.3.jar",
            "ant-1.10.17.jar",
        ] {
            std::fs::write(library.join(name), b"jar")?;
        }
        let found = GroovyHome::at(home.path());
        let runtime = found.as_ref().and_then(GroovyHome::runtime_jar);
        let core_jar = runtime
            .as_ref()
            .is_some_and(|path| path.ends_with("groovy-5.1.3.jar"));
        assert!(core_jar, "{runtime:?}");
        let every_jar = found.is_some_and(|home| home.class_path().ends_with('*'));
        assert!(every_jar);
        assert!(GroovyHome::at(&home.path().join("missing")).is_none());
        Ok(())
    }
}
