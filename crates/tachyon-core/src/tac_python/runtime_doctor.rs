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
        let roots = super::runtime_roots::RuntimeRoots::present(root)?;
        Ok(roots.then(|| Self::profile_row(profile)))
    }

    fn profile_row(profile: Option<super::RuntimeProfile>) -> Toolchain {
        let root = RuntimeAssets::cached_root();
        match profile {
            Some(_) => Self::inspect(&root),
            None => Self::row(&root, Err(super::RuntimeInventory::profile_missing())),
        }
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
