use super::{ArchiveRecipes, InstallContext, InstallPlan, InstallStep, NeededTool, ToolCommand};

pub(crate) struct TypeScriptRecipe;

impl TypeScriptRecipe {
    const TYPESCRIPT: &'static str = "7.0.2";
    const BUN: &'static str = "bun";

    pub(crate) fn plan(context: &InstallContext) -> InstallPlan {
        let installed =
            ArchiveRecipes::bun_folder(context).join(context.platform.executable(Self::BUN));
        let bun = context.program(Self::BUN, &installed);
        let package = format!("typescript@{}", Self::TYPESCRIPT);
        let command =
            ToolCommand::new(&bun, &["add", "--dev", &package]).in_folder(context.project.clone());
        let lines = vec![format!(
            "Bun adds TypeScript {} to this project's node_modules. Bun checks each package against its published integrity hash.",
            Self::TYPESCRIPT
        )];
        InstallPlan::automatic(
            NeededTool::TypeScript,
            vec![InstallStep::Run(command)],
            lines,
        )
    }
}
