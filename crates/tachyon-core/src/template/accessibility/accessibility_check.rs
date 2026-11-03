use super::ViewAudit;
use crate::annotation_layout::RuleSwitch;
use crate::compiler::PortablePath;
use crate::template::{TemplateFrontend, TemplateProgram};
use cap_std::fs::Dir;
use std::collections::BTreeSet;
use std::path::Path;
use tachyon_diagnostics::Diagnostic;

pub(crate) struct AccessibilityCheck;

impl AccessibilityCheck {
    const SWITCH: &'static str = "accessibility";
    const ON_BY_DEFAULT: bool = true;
    const VIEW_FILE: &'static str = "tac.html";
    const COMPONENT_ROOT: &'static str = "client/components";
    const COMPONENT_WORD_SEPARATOR: &'static str = "-";

    pub(crate) fn switch(project: &Dir) -> RuleSwitch {
        RuleSwitch::read_or(project, Self::SWITCH, Self::ON_BY_DEFAULT)
    }

    pub(crate) fn diagnostics<'a>(
        files: impl Iterator<Item = (&'a Path, &'a [u8])>,
        switch: RuleSwitch,
    ) -> Vec<Diagnostic> {
        let views: Vec<(&Path, &[u8])> = files.filter(|(path, _)| Self::is_view(path)).collect();
        let components: BTreeSet<String> = views
            .iter()
            .filter_map(|(path, _)| Self::component_name(path))
            .collect();
        views
            .iter()
            .filter(|(path, _)| switch.covers(path))
            .flat_map(|(path, bytes)| Self::view_diagnostics(path, bytes, &components))
            .collect()
    }

    pub(crate) fn program_diagnostics(program: &TemplateProgram) -> Vec<Diagnostic> {
        ViewAudit::diagnostics(program)
    }

    fn is_view(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == Self::VIEW_FILE)
    }

    fn component_name(path: &Path) -> Option<String> {
        let folder = path.parent()?.strip_prefix(Self::COMPONENT_ROOT).ok()?;
        let words: Vec<&str> = folder.iter().filter_map(|word| word.to_str()).collect();
        Some(words.join(Self::COMPONENT_WORD_SEPARATOR))
    }

    fn view_diagnostics(
        path: &Path,
        bytes: &[u8],
        components: &BTreeSet<String>,
    ) -> Vec<Diagnostic> {
        let source = String::from_utf8_lossy(bytes);
        TemplateFrontend::compile(&source, &PortablePath::of(path), components)
            .map(|program| ViewAudit::diagnostics(&program))
            .unwrap_or_default()
    }
}
