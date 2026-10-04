use crate::handler::readiness::RuntimeProbe;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlerRuntimePrograms {
    pub javascript: PathBuf,
    pub python: PathBuf,
    pub ruby: PathBuf,
}

impl Default for HandlerRuntimePrograms {
    fn default() -> Self {
        Self::from_lookup(|name| std::env::var_os(name))
    }
}

impl HandlerRuntimePrograms {
    pub(in crate::handler) const DEFAULT_JAVASCRIPT: &str = "node";
    pub(in crate::handler) const BUN_JAVASCRIPT: &str = "bun";
    #[cfg(windows)]
    pub(in crate::handler) const DEFAULT_PYTHON: &str = "python";
    #[cfg(not(windows))]
    pub(in crate::handler) const DEFAULT_PYTHON: &str = "python3";
    pub(in crate::handler) const DEFAULT_RUBY: &str = "ruby";

    pub(crate) fn from_lookup<F>(lookup: F) -> Self
    where
        F: Fn(&str) -> Option<OsString>,
    {
        Self {
            javascript: Self::runtime(
                &lookup,
                "YON_JAVASCRIPT_RUNTIME",
                Self::installed_javascript(&lookup),
            ),
            python: Self::runtime(&lookup, "YON_PYTHON_RUNTIME", Self::DEFAULT_PYTHON),
            ruby: Self::runtime(&lookup, "YON_RUBY_RUNTIME", Self::DEFAULT_RUBY),
        }
    }

    pub(in crate::handler) fn is_default_javascript(program: &Path) -> bool {
        [Self::DEFAULT_JAVASCRIPT, Self::BUN_JAVASCRIPT]
            .iter()
            .any(|name| program == Path::new(name))
    }

    pub(in crate::handler) fn default_javascript_for(program: &Path) -> &'static str {
        [Self::BUN_JAVASCRIPT, Self::DEFAULT_JAVASCRIPT]
            .into_iter()
            .find(|name| program == Path::new(name))
            .unwrap_or(Self::DEFAULT_JAVASCRIPT)
    }

    fn installed_javascript<F>(lookup: &F) -> &'static str
    where
        F: Fn(&str) -> Option<OsString>,
    {
        let paths = lookup("PATH").unwrap_or_default();
        let found = |name: &str| RuntimeProbe::on_path(&paths, Path::new(name));
        if !found(Self::DEFAULT_JAVASCRIPT) && found(Self::BUN_JAVASCRIPT) {
            return Self::BUN_JAVASCRIPT;
        }
        Self::DEFAULT_JAVASCRIPT
    }

    fn runtime<F>(lookup: &F, name: &str, fallback: &str) -> PathBuf
    where
        F: Fn(&str) -> Option<OsString>,
    {
        lookup(name)
            .filter(|value| !value.is_empty())
            .map_or_else(|| PathBuf::from(fallback), PathBuf::from)
    }
}
