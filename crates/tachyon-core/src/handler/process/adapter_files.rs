use crate::Failure;
use crate::failure::diagnostic;
use crate::handler::HandlerLanguage;
use std::fs;
use std::path::PathBuf;

pub(super) struct AdapterFiles {
    _directory: tempfile::TempDir,
    pub(super) runner: PathBuf,
}

impl AdapterFiles {
    pub(super) const JAVASCRIPT_RUNNER: &str = include_str!("../adapters/javascript_runner.mjs");
    pub(super) const PYTHON_RUNNER: &str = include_str!("../adapters/python_runner.py");
    pub(super) const RUBY_RUNNER: &str = include_str!("../adapters/ruby_runner.rb");

    pub(super) fn materialize(language: HandlerLanguage) -> Result<Self, Failure> {
        let directory = tempfile::Builder::new()
            .prefix("tachyon-handler-")
            .tempdir()
            .map_err(|error| Self::io_failure(&error))?;
        let (name, contents) = Self::runner_source(language);
        let runner = directory.path().join(name);
        fs::write(&runner, contents).map_err(|error| Self::io_failure(&error))?;
        Ok(Self {
            _directory: directory,
            runner,
        })
    }

    pub(super) fn io_failure(error: &std::io::Error) -> Failure {
        Failure::one(diagnostic(
            2101,
            format!("Cannot materialize the trusted handler adapter: {error}"),
            Some(String::from(
                "Check temporary-directory permissions and available storage.",
            )),
            None,
        ))
    }

    fn runner_source(language: HandlerLanguage) -> (&'static str, &'static str) {
        match language {
            HandlerLanguage::JavaScript | HandlerLanguage::TypeScript => {
                ("runner.mjs", Self::JAVASCRIPT_RUNNER)
            }
            HandlerLanguage::Python => ("runner.py", Self::PYTHON_RUNNER),
            HandlerLanguage::Ruby => ("runner.rb", Self::RUBY_RUNNER),
            HandlerLanguage::Direct => ("direct.unused", ""),
        }
    }
}
