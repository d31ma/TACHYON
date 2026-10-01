use super::{native_ui_capture::NativeUiCapture, native_ui_problem::NativeUiProblem};
use crate::{Failure, WebCompiler};
use cap_std::{ambient_authority, fs::Dir};
use std::path::Path;
use tachyon_contracts::NativeTarget;

pub(super) struct NativeUiProfile;

impl NativeUiProfile {
    pub(super) fn prepare(enabled: bool, root: &Path, target: NativeTarget) -> Result<(), Failure> {
        Self::selected(enabled, root, target)
    }

    fn selected(enabled: bool, root: &Path, target: NativeTarget) -> Result<(), Failure> {
        if !enabled {
            return Ok(());
        }
        Self::prepare_root(root, target)
    }

    fn prepare_root(root: &Path, target: NativeTarget) -> Result<(), Failure> {
        Self::target(target)?;
        let root = Dir::open_ambient_dir(root, ambient_authority())
            .map_err(|error| NativeUiProblem::io(&error))?;
        let captured = NativeUiCapture::read(&root)?;
        Self::registration(&captured)?;
        Self::worker(&captured)?;
        ["tachyon-sw.js", "tachyon-sw.js.br", "tachyon-sw.js.gz"]
            .iter()
            .try_for_each(|path| Self::remove(&root, &captured, path))
    }

    fn registration(captured: &NativeUiCapture) -> Result<(), Failure> {
        let expected = WebCompiler::native_worker_registration().as_bytes();
        NativeUiProblem::require(
            captured
                .bytes
                .get(".tachyon/register-sw.js")
                .map(Vec::as_slice)
                == Some(expected),
            "Native UI candidate requires the compiler-owned native-aware worker registration.",
        )
    }

    fn worker(captured: &NativeUiCapture) -> Result<(), Failure> {
        NativeUiProblem::require(
            captured.bytes.contains_key("tachyon-sw.js"),
            "Native UI candidate requires the compiler-generated worker before profiling.",
        )
    }

    fn remove(root: &Dir, captured: &NativeUiCapture, path: &str) -> Result<(), Failure> {
        if captured.bytes.contains_key(path) {
            root.remove_file(path)
                .map_err(|error| NativeUiProblem::io(&error))?;
        }
        Ok(())
    }

    pub(super) fn target(target: NativeTarget) -> Result<(), Failure> {
        NativeUiProblem::require(
            target == NativeTarget::Macos,
            "Native UI candidate profile currently supports macOS only.",
        )
    }
}
