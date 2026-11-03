#![allow(clippy::expect_used)]

use super::KnownToolchains;
use crate::tool_setup::NeededTool;
use std::fs;
use std::path::{Path, PathBuf};

struct TypeScriptInstallFixture;

impl TypeScriptInstallFixture {
    fn file(path: &Path) -> PathBuf {
        fs::create_dir_all(path.parent().expect("parent folder")).expect("fixture folder");
        fs::write(path, b"").expect("fixture file");
        path.to_path_buf()
    }

    fn npm_project_on_windows(project: &Path) -> PathBuf {
        let node_modules = project.join("node_modules");
        Self::file(&node_modules.join(".bin/tsc.cmd"));
        Self::file(&node_modules.join("@typescript/typescript-win32-x64/lib/tsc.exe"))
    }
}

#[test]
fn doctor_probes_the_native_compiler_behind_a_project_cmd_shim() {
    let project = tempfile::tempdir().expect("project");
    let native = TypeScriptInstallFixture::npm_project_on_windows(project.path());

    let program = KnownToolchains::typescript_program(project.path());

    assert_eq!(Path::new(&program), native);
}

#[test]
fn doctor_names_the_project_compiler_so_tool_setup_can_install_it() {
    let project = tempfile::tempdir().expect("project");
    TypeScriptInstallFixture::npm_project_on_windows(project.path());

    let toolchain = KnownToolchains::typescript(project.path());

    assert_eq!(toolchain.command, "node_modules/.bin/tsc");
    assert_eq!(
        NeededTool::for_command(&toolchain.command),
        Some(NeededTool::TypeScript)
    );
}
