use super::Module;
use crate::Failure;
use std::path::Path;

pub(crate) struct PageParts;

impl PageParts {
    const MARKER: &'static str = "//tachyon:part ";
    const SEPARATOR: &'static str = "\n//tachyon:part ";
    const EXTENSION: &'static str = "cs";
    const USING: &'static str = "using ";

    pub(crate) fn assemble(
        snapshot_root: &Path,
        entry: &Path,
        source: &str,
        parts: bool,
    ) -> Result<Option<String>, Failure> {
        Module::assembled(snapshot_root, entry, source, ("tac", false, parts))
    }

    pub(super) fn marked(modules: Vec<Module>, wanted: bool) -> Vec<Module> {
        modules
            .into_iter()
            .map(|module| Self::with_marker(module, wanted))
            .collect()
    }

    fn with_marker(module: Module, wanted: bool) -> Module {
        let marker = format!(
            "{}{}\n",
            Self::MARKER,
            module.path.to_string_lossy().replace('\\', "/")
        );
        let marker = [String::new(), marker][usize::from(wanted)].clone();
        Module {
            source: format!("{marker}{}", module.source),
            ..module
        }
    }

    pub(crate) fn split(program: &str, entry: &str) -> Vec<(String, String)> {
        let mut segments = program.split(Self::SEPARATOR);
        let first = segments.next().unwrap_or_default();
        std::iter::once((entry.to_owned(), first.to_owned()))
            .chain(segments.map(Self::labelled))
            .collect()
    }

    pub(crate) fn program(path: &Path, parts: &[String]) -> Result<String, Failure> {
        super::hoist_imports(path, &parts.join("\n"), Self::EXTENSION)
    }

    pub(crate) fn usings(program: &str) -> String {
        program
            .lines()
            .take_while(|line| line.starts_with(Self::USING))
            .flat_map(|line| [line, "\n"])
            .collect()
    }

    fn labelled(segment: &str) -> (String, String) {
        let (path, text) = segment.split_once('\n').unwrap_or((segment, ""));
        (path.trim().to_owned(), format!("\n{text}"))
    }
}
