use super::NativePackageFolder;
use std::path::Path;

fn refused(path: &str, source: &str) -> String {
    NativePackageFolder::check_file(Path::new(path), source.as_bytes())
        .expect_err("refused")
        .to_string()
}

#[test]
fn plain_go_packages_in_the_native_folder_need_no_role_marker() {
    let source = "package git\n\ntype Repository struct{ Path string }\n";

    let checked = NativePackageFolder::check_file(
        Path::new("server/packages/helpers/git/repository.go"),
        source.as_bytes(),
    );

    assert!(checked.is_ok());
}

#[test]
fn the_module_file_names_the_import_path() {
    let module = "// app packages\nmodule example.com/heimdall/packages // root\n\ngo 1.27\n";

    assert_eq!(
        NativePackageFolder::module_path(module).as_deref(),
        Some("example.com/heimdall/packages")
    );
}

#[test]
fn a_module_file_without_a_module_line_is_refused() {
    let failure =
        NativePackageFolder::check_file(Path::new("server/packages/go.mod"), b"go 1.27\n")
            .expect_err("no module line");

    assert_eq!(failure.diagnostics()[0].code.number(), 1310);
}

#[test]
fn a_module_file_may_require_vendored_modules() {
    let module = "module example.com/app\n\ngo 1.27\n\nrequire github.com/acme/kit v1.0.0\n\nrequire (\n\tgopkg.in/yaml.v3 v3.0.1\n\tgithub.com/acme/base v1.2.0 // indirect\n)\n";

    let checked =
        NativePackageFolder::check_file(Path::new("server/packages/go.mod"), module.as_bytes());

    assert!(checked.is_ok(), "{checked:?}");
}

#[test]
fn a_module_file_with_a_replace_line_is_refused() {
    let module =
        "module example.com/app\n\ngo 1.27\n\nreplace(\n\tgithub.com/acme/kit => ../kit\n)\n";

    assert!(refused("server/packages/go.mod", module).contains("found `replace`"));
}

#[test]
fn vendored_code_is_not_checked_as_native_code() {
    let source = "package kit\n\n// @yon::Service\ntype KitService struct{}\n";

    let checked = NativePackageFolder::check_file(
        Path::new("server/packages/vendor/github.com/acme/kit/go.mod"),
        source.as_bytes(),
    )
    .and_then(|()| {
        NativePackageFolder::check_file(
            Path::new("server/packages/vendor/github.com/acme/kit/service.go"),
            source.as_bytes(),
        )
    });

    assert!(checked.is_ok(), "{checked:?}");
}

#[test]
fn the_handler_program_folder_is_reserved() {
    let source = "package yonhandler\n";

    assert!(refused("server/packages/yonhandler/main.go", source).contains("reserved"));
}

#[test]
fn a_nested_module_file_is_refused() {
    assert!(refused("server/packages/git/go.mod", "module x\n").contains("only module file"));
}

#[test]
fn annotated_go_code_in_the_native_folder_is_refused() {
    let source = "package git\n\n// @yon::Service\ntype GitService struct{}\n";

    assert!(refused("server/packages/git/GitService.go", source).contains("annotation"));
}

#[test]
fn another_yon_language_in_the_native_folder_is_refused() {
    assert!(refused("server/packages/tools/Tool.py", "x = 1\n").contains(".py file"));
}

#[test]
fn assets_in_the_native_folder_are_accepted() {
    let checked =
        NativePackageFolder::check_file(Path::new("server/packages/schema/item.json"), b"{}");

    assert!(checked.is_ok());
}

#[test]
fn native_code_without_a_module_file_is_refused() {
    let paths = [
        Path::new("server/controllers/ItemController.go"),
        Path::new("server/packages/git/repository.go"),
    ];

    let checked = NativePackageFolder::check_module(paths.into_iter());

    assert!(
        checked
            .expect_err("no module")
            .to_string()
            .contains("no go.mod")
    );
}

#[test]
fn native_code_with_its_module_file_is_accepted() {
    let paths = [
        Path::new("server/packages/git/repository.go"),
        Path::new("server/packages/go.mod"),
    ];

    assert!(NativePackageFolder::check_module(paths.into_iter()).is_ok());
}
