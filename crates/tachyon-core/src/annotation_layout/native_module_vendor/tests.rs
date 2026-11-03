use super::NativeModuleVendor;
use std::path::Path;

struct VendorFiles;

impl VendorFiles {
    const REQUIRING: &str =
        "module example.com/app\n\ngo 1.27\n\nrequire (\n\tgopkg.in/yaml.v3 v3.0.1\n)\n";
    const PLAIN: &str = "module example.com/app\n\ngo 1.27\n";
    const SUMS: &str = "gopkg.in/yaml.v3 v3.0.1 h1:abc=\n";
    const LIST: &str = "# gopkg.in/yaml.v3 v3.0.1\n## explicit\ngopkg.in/yaml.v3\n";

    fn checked(files: &[(&str, &str)]) -> Result<(), crate::Failure> {
        NativeModuleVendor::check(
            files
                .iter()
                .map(|(path, text)| (Path::new(*path), text.as_bytes())),
        )
    }
}

#[test]
fn a_module_without_require_lines_needs_no_vendor_folder() {
    let checked = VendorFiles::checked(&[("server/packages/go.mod", VendorFiles::PLAIN)]);

    assert!(checked.is_ok(), "{checked:?}");
}

#[test]
fn a_vendored_module_with_its_sums_and_list_is_accepted() {
    let checked = VendorFiles::checked(&[
        ("server/packages/go.mod", VendorFiles::REQUIRING),
        ("server/packages/go.sum", VendorFiles::SUMS),
        ("server/packages/vendor/modules.txt", VendorFiles::LIST),
    ]);

    assert!(checked.is_ok(), "{checked:?}");
}

#[test]
fn a_required_module_without_a_vendor_folder_fails_with_ty1368() {
    let failure = VendorFiles::checked(&[
        ("server/packages/go.mod", VendorFiles::REQUIRING),
        ("server/packages/go.sum", VendorFiles::SUMS),
    ])
    .expect_err("no vendor folder");

    let rendered = failure.to_string();
    assert_eq!(failure.diagnostics()[0].code.number(), 1368);
    assert!(
        rendered.contains("server/packages/vendor/modules.txt is missing"),
        "{rendered}"
    );
    assert!(rendered.contains("go mod vendor"), "{rendered}");
}

#[test]
fn a_required_module_without_go_sum_fails_with_ty1368() {
    let failure = VendorFiles::checked(&[
        ("server/packages/go.mod", VendorFiles::REQUIRING),
        ("server/packages/vendor/modules.txt", VendorFiles::LIST),
    ])
    .expect_err("no go.sum");

    assert!(
        failure
            .to_string()
            .contains("server/packages/go.sum is missing")
    );
}

#[test]
fn only_files_below_the_vendor_folder_are_vendored() {
    assert!(NativeModuleVendor::contains(Path::new(
        "server/packages/vendor/gopkg.in/yaml.v3/yaml.go"
    )));
    assert!(!NativeModuleVendor::contains(Path::new(
        "server/packages/vendors/list.go"
    )));
    assert!(!NativeModuleVendor::contains(Path::new(
        "server/controllers/vendor/ItemController.go"
    )));
}
