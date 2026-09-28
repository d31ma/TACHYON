use std::ffi::OsString;
use std::path::PathBuf;

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
            javascript: Self::runtime(&lookup, "YON_JAVASCRIPT_RUNTIME", Self::DEFAULT_JAVASCRIPT),
            python: Self::runtime(&lookup, "YON_PYTHON_RUNTIME", Self::DEFAULT_PYTHON),
            ruby: Self::runtime(&lookup, "YON_RUBY_RUNTIME", Self::DEFAULT_RUBY),
        }
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
