use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(super) struct TeaVmToolchain {
    compiler: PathBuf,
    plugins: Vec<PathBuf>,
}

impl TeaVmToolchain {
    const HOME: &'static str = "TAC_TEAVM_HOME";
    const MANAGED_FOLDER: &'static str = "teavm-0.15.0";
    const COMPILER_SUFFIX: &'static str = "-all.jar";
    const LIBRARY_EXTENSION: &'static str = "jar";

    pub(super) fn find() -> Option<Self> {
        let jars = Self::jars(&Self::home());
        let (compilers, plugins): (Vec<PathBuf>, Vec<PathBuf>) =
            jars.into_iter().partition(|jar| Self::is_compiler(jar));
        let compiler = compilers.into_iter().next()?;
        Some(Self { compiler, plugins })
    }

    pub(super) fn home() -> PathBuf {
        Self::home_from(std::env::var_os(Self::HOME))
    }

    fn home_from(variable: Option<OsString>) -> PathBuf {
        variable
            .filter(|home| !home.is_empty())
            .map_or_else(Self::managed_folder, PathBuf::from)
    }

    pub(super) fn managed_folder() -> PathBuf {
        crate::cache::root().join(Self::MANAGED_FOLDER)
    }

    pub(super) fn compiler(&self) -> &Path {
        &self.compiler
    }

    pub(super) fn tool_class_path(&self) -> OsString {
        let entries = self.plugins.iter().chain([&self.compiler]);
        std::env::join_paths(entries).unwrap_or_default()
    }

    pub(super) fn identity(&self) -> Vec<u8> {
        let names = self.plugins.iter().chain([&self.compiler]);
        names
            .map(|jar| jar.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n")
            .into_bytes()
    }

    fn jars(home: &Path) -> Vec<PathBuf> {
        let entries = std::fs::read_dir(home).into_iter().flatten().flatten();
        let mut jars: Vec<PathBuf> = entries
            .map(|entry| entry.path())
            .filter(|path| Self::is_library(path))
            .collect();
        jars.sort();
        jars
    }

    fn is_library(path: &Path) -> bool {
        path.extension() == Some(std::ffi::OsStr::new(Self::LIBRARY_EXTENSION))
    }

    fn is_compiler(jar: &Path) -> bool {
        jar.to_string_lossy().ends_with(Self::COMPILER_SUFFIX)
    }
}

#[cfg(test)]
mod tests {
    use super::TeaVmToolchain;
    use std::ffi::OsString;
    use std::path::PathBuf;

    #[test]
    fn without_tac_teavm_home_the_jars_come_from_the_tachyon_cache() {
        let managed = crate::cache::root().join("teavm-0.15.0");
        assert_eq!(TeaVmToolchain::home_from(None), managed);
        assert_eq!(TeaVmToolchain::home_from(Some(OsString::new())), managed);
        assert_eq!(
            TeaVmToolchain::home_from(Some(OsString::from("/opt/teavm"))),
            PathBuf::from("/opt/teavm")
        );
    }
}
