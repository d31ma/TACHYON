use super::{
    ArchiveRecipes, DotnetRecipes, EmscriptenRecipe, InstallContext, InstallPlan, NeededTool,
    RustRecipes, SdkmanRecipes, SystemRecipes, TinyGoRecipe, TypeScriptRecipe,
};

pub struct InstallRecipes;

impl InstallRecipes {
    #[must_use]
    pub fn plan(tool: NeededTool, context: &InstallContext) -> InstallPlan {
        Self::plan_for(tool, context)
    }

    #[must_use]
    pub fn plans(tools: &[NeededTool], context: &InstallContext) -> Vec<InstallPlan> {
        tools
            .iter()
            .map(|tool| Self::plan_for(*tool, context))
            .collect()
    }

    fn plan_for(tool: NeededTool, context: &InstallContext) -> InstallPlan {
        match tool {
            NeededTool::Bun => ArchiveRecipes::bun(context),
            NeededTool::Go => ArchiveRecipes::go(context),
            NeededTool::TinyGo => TinyGoRecipe::plan(context),
            NeededTool::Binaryen => ArchiveRecipes::binaryen(context),
            NeededTool::Dart => ArchiveRecipes::dart(context),
            NeededTool::TypeScript => TypeScriptRecipe::plan(context),
            NeededTool::Emscripten => EmscriptenRecipe::plan(context),
            NeededTool::Java => SdkmanRecipes::java(context),
            NeededTool::Groovy => SdkmanRecipes::groovy(context),
            NeededTool::Kotlin => SdkmanRecipes::kotlin(context),
            NeededTool::Gradle => SdkmanRecipes::gradle(context),
            NeededTool::DotNet => DotnetRecipes::dotnet(context),
            NeededTool::WasmTools => DotnetRecipes::wasm_tools(context),
            NeededTool::Rust => RustRecipes::rust(context),
            NeededTool::WasmTarget => RustRecipes::wasm_target(context),
            NeededTool::Python
            | NeededTool::Php
            | NeededTool::Ruby
            | NeededTool::Lua
            | NeededTool::CCompiler
            | NeededTool::Xcode
            | NeededTool::AndroidSdk
            | NeededTool::LinuxDesktop => SystemRecipes::plan(tool, context),
        }
    }
}
