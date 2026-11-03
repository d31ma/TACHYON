#![allow(clippy::expect_used)]

use super::TypeScriptCompiler;
use std::fs;
use std::path::{Path, PathBuf};

struct TypeScriptProgramFixture;

impl TypeScriptProgramFixture {
    fn file(path: &Path) -> PathBuf {
        fs::create_dir_all(path.parent().expect("parent folder")).expect("fixture folder");
        fs::write(path, b"").expect("fixture file");
        path.to_path_buf()
    }

    fn native_compiler(node_modules: &Path) -> PathBuf {
        let native = node_modules.join("@typescript/typescript-win32-x64/lib/tsc.exe");
        Self::file(&native)
    }
}

#[cfg(windows)]
#[test]
fn a_project_with_only_a_cmd_shim_runs_the_native_compiler_of_its_typescript_package() {
    let project = tempfile::tempdir().expect("project");
    let node_modules = project.path().join("node_modules");
    TypeScriptProgramFixture::file(&node_modules.join(".bin/tsc.cmd"));
    TypeScriptProgramFixture::file(&node_modules.join("typescript/package.json"));
    let native = TypeScriptProgramFixture::native_compiler(&node_modules);

    assert_eq!(TypeScriptCompiler::program(project.path()), native);
}

#[test]
fn a_global_npm_shim_on_the_search_path_runs_the_nested_native_compiler() {
    let project = tempfile::tempdir().expect("project");
    let prefix = tempfile::tempdir().expect("npm prefix");
    TypeScriptProgramFixture::file(&prefix.path().join("tsc.cmd"));
    let package_modules = prefix.path().join("node_modules/typescript/node_modules");
    let native = TypeScriptProgramFixture::native_compiler(&package_modules);
    let search_path = std::env::join_paths([prefix.path()]).expect("search path");

    let program = TypeScriptCompiler::windows_program(project.path(), &search_path);

    assert_eq!(program, native);
}

#[test]
fn a_native_compiler_earlier_on_the_search_path_is_used_first() {
    let project = tempfile::tempdir().expect("project");
    let standalone = tempfile::tempdir().expect("standalone compiler");
    let prefix = tempfile::tempdir().expect("npm prefix");
    let first = TypeScriptProgramFixture::file(&standalone.path().join("tsc.exe"));
    TypeScriptProgramFixture::file(&prefix.path().join("tsc.cmd"));
    TypeScriptProgramFixture::native_compiler(&prefix.path().join("node_modules"));
    let search_path =
        std::env::join_paths([standalone.path(), prefix.path()]).expect("search path");

    let program = TypeScriptCompiler::windows_program(project.path(), &search_path);

    assert_eq!(program, first);
}

#[test]
fn a_shim_without_a_native_compiler_is_never_run_as_a_command_script() {
    let project = tempfile::tempdir().expect("project");
    let bin = project.path().join("node_modules/.bin");
    TypeScriptProgramFixture::file(&bin.join("tsc"));
    TypeScriptProgramFixture::file(&bin.join("tsc.cmd"));
    let search_path = std::env::join_paths([bin.as_path()]).expect("search path");

    let program = TypeScriptCompiler::windows_program(project.path(), &search_path);

    assert_eq!(program, PathBuf::from("tsc"));
}
