use super::{InstallContext, InstallPlan, InstallStep, NeededTool, ToolCommand, UnpackSteps};
use std::path::Path;

pub(crate) struct DotnetRecipes;

impl DotnetRecipes {
    const DOTNET: &'static str = "9.0.317";
    const PROGRAM: &'static str = "dotnet";
    const FOLDER: &'static str = ".dotnet";
    const SCRIPT: &'static str = "dotnet-install.sh";
    const WINDOWS_SCRIPT: &'static str = "dotnet-install.ps1";

    pub(crate) fn dotnet(context: &InstallContext) -> InstallPlan {
        let into = context.in_home(Self::FOLDER);
        let steps = Self::install_steps(context, &into);
        let lines = vec![
            format!(
                "The .NET SDK {} goes into {}.",
                Self::DOTNET,
                into.display()
            ),
            String::from(
                "Microsoft publishes no checksum for its install script. The script downloads the SDK from Microsoft's own servers.",
            ),
            UnpackSteps::path_line(context, &into),
        ];
        InstallPlan::automatic(NeededTool::DotNet, steps, lines)
    }

    pub(crate) fn wasm_tools(context: &InstallContext) -> InstallPlan {
        let installed = context
            .in_home(Self::FOLDER)
            .join(context.platform.executable(Self::PROGRAM));
        let dotnet = context.program(Self::PROGRAM, &installed);
        let command = ToolCommand::new(&dotnet, &["workload", "install", "wasm-tools"]);
        let lines = vec![String::from(
            "dotnet adds the wasm-tools workload, which builds C# for the browser.",
        )];
        InstallPlan::automatic(
            NeededTool::WasmTools,
            vec![InstallStep::Run(command)],
            lines,
        )
    }

    fn install_steps(context: &InstallContext, into: &Path) -> Vec<InstallStep> {
        if context.platform.is_windows() {
            return Self::windows_steps(context, into);
        }
        Self::unix_steps(context, into)
    }

    fn windows_steps(context: &InstallContext, into: &Path) -> Vec<InstallStep> {
        let script = context.download(Self::WINDOWS_SCRIPT);
        let mut steps = UnpackSteps::unverified_download(
            context,
            &format!("https://dot.net/v1/{}", Self::WINDOWS_SCRIPT),
            Self::WINDOWS_SCRIPT,
        );
        steps.push(InstallStep::Run(ToolCommand::new(
            "powershell",
            &[
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
                &script.to_string_lossy(),
                "-Version",
                Self::DOTNET,
                "-InstallDir",
                &into.to_string_lossy(),
            ],
        )));
        steps
    }

    fn unix_steps(context: &InstallContext, into: &Path) -> Vec<InstallStep> {
        let script = context.download(Self::SCRIPT);
        let mut steps = UnpackSteps::unverified_download(
            context,
            &format!("https://dot.net/v1/{}", Self::SCRIPT),
            Self::SCRIPT,
        );
        steps.push(InstallStep::Run(ToolCommand::new(
            "bash",
            &[
                &script.to_string_lossy(),
                "--version",
                Self::DOTNET,
                "--install-dir",
                &into.to_string_lossy(),
            ],
        )));
        steps
    }
}
