use super::NeededTool;
use crate::doctor::{CommandProbe, DoctorReport, TargetToolchains, Toolchain};
use crate::{AppTarget, Failure, ProjectChoices, TacLanguage};
use std::path::Path;

pub struct ToolCheck;

impl ToolCheck {
    const JAVASCRIPT_RUNTIMES: [&'static str; 2] = ["bun", "node"];
    const VERSION: [&'static str; 1] = ["--version"];
    const JAVASCRIPT_SERVICES: [TacLanguage; 4] = [
        TacLanguage::JavaScript,
        TacLanguage::TypeScript,
        TacLanguage::Swift,
        TacLanguage::Kotlin,
    ];

    pub fn missing(root: &Path, choices: &ProjectChoices) -> Result<Vec<NeededTool>, Failure> {
        let report = DoctorReport::check(root)?;
        let mut tools = Self::from_report(&report.toolchains);
        tools.extend(Self::javascript_runtime(choices));
        tools.extend(Self::target_tools(&choices.targets));
        tools.sort();
        tools.dedup();
        Ok(tools)
    }

    pub fn missing_for_project(root: &Path) -> Result<Vec<NeededTool>, Failure> {
        let report = DoctorReport::check(root)?;
        let mut tools = Self::from_report(&report.toolchains);
        tools.sort();
        tools.dedup();
        Ok(tools)
    }

    fn from_report(toolchains: &[Toolchain]) -> Vec<NeededTool> {
        toolchains
            .iter()
            .filter(|toolchain| !toolchain.state.is_ready())
            .filter_map(|toolchain| NeededTool::for_command(&toolchain.command))
            .collect()
    }

    fn javascript_runtime(choices: &ProjectChoices) -> Option<NeededTool> {
        if !Self::runs_javascript_tests(choices) {
            return None;
        }
        (!Self::javascript_runtime_found()).then_some(NeededTool::Bun)
    }

    fn runs_javascript_tests(choices: &ProjectChoices) -> bool {
        let services = choices
            .tac
            .iter()
            .any(|language| Self::JAVASCRIPT_SERVICES.contains(language));
        [services, !choices.features.is_empty()].contains(&true)
    }

    fn javascript_runtime_found() -> bool {
        Self::JAVASCRIPT_RUNTIMES
            .iter()
            .any(|program| CommandProbe::output(program, &Self::VERSION).is_some())
    }

    fn target_tools(targets: &[AppTarget]) -> Vec<NeededTool> {
        Self::from_report(&TargetToolchains::for_targets(targets))
    }
}
