use super::runtime_assets::RuntimeAssets;
use crate::doctor::{Toolchain, ToolchainState};
use std::path::Path;

pub(crate) struct RuntimeDoctor;

impl RuntimeDoctor {
    pub(crate) fn inspect(root: &Path) -> Toolchain {
        Self::row(root, RuntimeAssets::read(root))
    }

    fn row(root: &Path, result: Result<Vec<(String, Vec<u8>)>, crate::Failure>) -> Toolchain {
        Toolchain {
            requirement: String::from(
                "Python services on the web: pinned Pyodide 314.0.7 / CPython 3.14.2 / ABI 2026_0",
            ),
            command: String::from("pyodide.asm.wasm"),
            state: Self::state(root, result),
            install: format!(
                "Provide the five pinned core assets at '{}'; automatic runtime/package installation is disabled. Check the exact inventory in docs/queue/NEXT/PYTHON-TAC.md.",
                root.display()
            ),
            required: true,
        }
    }

    fn state(
        root: &Path,
        result: Result<Vec<(String, Vec<u8>)>, crate::Failure>,
    ) -> ToolchainState {
        match result {
            Ok(_) => {
                ToolchainState::Ready(format!("five core assets verified at {}", root.display()))
            }
            Err(failure) => ToolchainState::Incomplete(failure.to_string()),
        }
    }
}
