use std::path::{Path, PathBuf};

pub(crate) struct GroovyHome {
    library: PathBuf,
}

impl GroovyHome {
    const HOME_VARIABLE: &'static str = "GROOVY_HOME";
    const PATH_VARIABLE: &'static str = "PATH";
    const LAUNCHERS: [&'static str; 2] = ["groovyc", "groovyc.bat"];
    const LIBRARY_FOLDER: &'static str = "lib";
    const RUNTIME_PREFIX: &'static str = "groovy-";
    const JAR_SUFFIX: &'static str = ".jar";
    const EVERY_JAR: &'static str = "*";

    pub(crate) fn find() -> Option<Self> {
        if let Some(home) = std::env::var_os(Self::HOME_VARIABLE) {
            return Self::at(Path::new(&home));
        }
        let launcher = Self::launcher_on_path()?;
        let launcher = std::fs::canonicalize(launcher).ok()?;
        let home = launcher.parent()?.parent()?;
        Self::at(home)
    }

    pub(crate) fn runtime_jar(&self) -> Option<PathBuf> {
        let entries = std::fs::read_dir(&self.library).ok()?;
        let mut found = Vec::new();
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if Self::is_runtime_jar(&name) {
                found.push(entry.path());
            }
        }
        found.sort();
        found.pop()
    }

    pub(crate) fn class_path(&self) -> String {
        self.library
            .join(Self::EVERY_JAR)
            .to_string_lossy()
            .into_owned()
    }

    fn at(home: &Path) -> Option<Self> {
        let library = home.join(Self::LIBRARY_FOLDER);
        if !library.is_dir() {
            return None;
        }
        Some(Self { library })
    }

    fn launcher_on_path() -> Option<PathBuf> {
        let path = std::env::var_os(Self::PATH_VARIABLE)?;
        for folder in std::env::split_paths(&path) {
            for launcher in Self::LAUNCHERS {
                let candidate = folder.join(launcher);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        None
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
