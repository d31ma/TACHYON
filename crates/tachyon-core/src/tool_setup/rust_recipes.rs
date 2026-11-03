use super::{InstallContext, InstallPlan, InstallStep, NeededTool, ToolCommand, UnpackSteps};

pub(crate) struct RustRecipes;

impl RustRecipes {
    const RUSTUP: &'static str = "1.29.0";
    const RUST: &'static str = "1.97.1";
    const RUSTUP_PROGRAM: &'static str = "rustup";
    const CARGO_BIN: &'static str = ".cargo/bin";
    const WASM_TARGET: &'static str = "wasm32-unknown-unknown";

    pub(crate) fn rust(context: &InstallContext) -> InstallPlan {
        let installer = context.platform.executable("rustup-init");
        let url = format!(
            "https://static.rust-lang.org/rustup/archive/{}/{}/{installer}",
            Self::RUSTUP,
            context.platform.names().rust
        );
        let mut steps =
            UnpackSteps::verified_download(context, &url, &format!("{url}.sha256"), &installer);
        let program = context.download(&installer).to_string_lossy().into_owned();
        steps.extend(Self::executable_steps(context, &program));
        steps.push(InstallStep::Run(ToolCommand::new(
            &program,
            &[
                "-y",
                "--no-modify-path",
                "--profile",
                "minimal",
                "--default-toolchain",
                Self::RUST,
                "--target",
                Self::WASM_TARGET,
            ],
        )));
        let lines = vec![
            format!(
                "rustup {} installs Rust {} into {}.",
                Self::RUSTUP,
                Self::RUST,
                context.in_home(".rustup").display()
            ),
            UnpackSteps::path_line(context, &context.in_home(Self::CARGO_BIN)),
        ];
        InstallPlan::automatic(NeededTool::Rust, steps, lines)
    }

    pub(crate) fn wasm_target(context: &InstallContext) -> InstallPlan {
        let installed = context
            .in_home(Self::CARGO_BIN)
            .join(context.platform.executable(Self::RUSTUP_PROGRAM));
        let rustup = context.program(Self::RUSTUP_PROGRAM, &installed);
        let command = ToolCommand::new(&rustup, &["target", "add", Self::WASM_TARGET]);
        let lines = vec![String::from(
            "rustup adds the WebAssembly target to your Rust toolchain.",
        )];
        InstallPlan::automatic(
            NeededTool::WasmTarget,
            vec![InstallStep::Run(command)],
            lines,
        )
    }

    fn executable_steps(context: &InstallContext, program: &str) -> Vec<InstallStep> {
        if context.platform.is_windows() {
            return Vec::new();
        }
        vec![InstallStep::Run(ToolCommand::new(
            "chmod",
            &["+x", program],
        ))]
    }
}
