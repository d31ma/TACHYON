use super::runtime_assets::RuntimeAssets;
use crate::doctor::{Toolchain, ToolchainState};
use std::path::Path;

pub(crate) struct RuntimeDoctor;

impl RuntimeDoctor {
    pub(crate) fn inspect(root: &Path) -> Toolchain {
        Self::row(root, RuntimeAssets::read(root))
    }

    pub(crate) fn for_project(root: &Path) -> Result<Option<Toolchain>, crate::Failure> {
        let profile = super::runtime_profile::RuntimeProfile::for_project(root)?;
        Self::selected_project(root, profile)
    }

    fn selected_project(
        root: &Path,
        profile: Option<super::runtime_profile::RuntimeProfile>,
    ) -> Result<Option<Toolchain>, crate::Failure> {
        if profile.is_none() {
            return Ok(None);
        }
        Ok(super::runtime_roots::RuntimeRoots::present(root)?
            .then(|| Self::inspect(&RuntimeAssets::cached_root())))
    }

    fn row(root: &Path, result: Result<Vec<(String, Vec<u8>)>, crate::Failure>) -> Toolchain {
        Toolchain {
            requirement: format!(
                "Python services on the web: pinned Pyodide {} / CPython 3.14.2 / ABI {}",
                RuntimeAssets::VERSION,
                RuntimeAssets::ABI
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
