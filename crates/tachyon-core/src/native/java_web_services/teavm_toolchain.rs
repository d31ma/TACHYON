use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(super) struct TeaVmToolchain {
    compiler: PathBuf,
    plugins: Vec<PathBuf>,
}

impl TeaVmToolchain {
    const HOME: &'static str = "TAC_TEAVM_HOME";
    const COMPILER_SUFFIX: &'static str = "-all.jar";
    const LIBRARY_EXTENSION: &'static str = "jar";

    pub(super) fn find() -> Option<Self> {
        let home = std::env::var_os(Self::HOME).map(PathBuf::from)?;
        let jars = Self::jars(&home);
        let (compilers, plugins): (Vec<PathBuf>, Vec<PathBuf>) =
            jars.into_iter().partition(|jar| Self::is_compiler(jar));
        let compiler = compilers.into_iter().next()?;
        Some(Self { compiler, plugins })
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
