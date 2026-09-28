use super::NeededTool;
use crate::doctor::{CommandProbe, DoctorReport, Toolchain};
use crate::{AppTarget, Failure, ProjectChoices, TacLanguage};
use std::path::Path;

pub struct ToolCheck;

impl ToolCheck {
    const JAVASCRIPT_RUNTIMES: [&'static str; 2] = ["bun", "node"];
    const VERSION: [&'static str; 1] = ["--version"];
    const ANDROID_VARIABLES: [&'static str; 2] = ["ANDROID_HOME", "ANDROID_SDK_ROOT"];
    const WEBKIT: [&'static str; 2] = ["--exists", "webkit2gtk-4.1"];
    const SWIFT: [&'static str; 2] = ["--find", "swiftc"];
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
        targets
            .iter()
            .flat_map(|target| Self::tools_for_target(*target))
            .copied()
            .filter(|tool| !Self::tool_found(*tool))
            .collect()
    }

    fn tools_for_target(target: AppTarget) -> &'static [NeededTool] {
        match target {
            AppTarget::Web => &[],
            AppTarget::Macos | AppTarget::Ios => &[NeededTool::Xcode],
            AppTarget::Android => &[NeededTool::Java, NeededTool::Gradle, NeededTool::AndroidSdk],
            AppTarget::Linux => &[NeededTool::CCompiler, NeededTool::LinuxDesktop],
            AppTarget::Windows => &[NeededTool::DotNet],
        }
    }

    fn tool_found(tool: NeededTool) -> bool {
        match tool {
            NeededTool::AndroidSdk => Self::android_sdk_found(),
            NeededTool::LinuxDesktop => CommandProbe::output("pkg-config", &Self::WEBKIT).is_some(),
            NeededTool::Xcode => CommandProbe::output("xcrun", &Self::SWIFT).is_some(),
            other => CommandProbe::output(other.command(), &Self::VERSION).is_some(),
        }
    }

    fn android_sdk_found() -> bool {
        Self::ANDROID_VARIABLES
            .iter()
            .any(|name| std::env::var_os(name).is_some())
    }
}
