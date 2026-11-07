use super::GoNativeModule;
use std::path::Path;

struct NativeProject;

impl NativeProject {
    const FILES: [(&str, &str); 4] = [
        (
            "server/packages/go.mod",
            "module example.com/shop/packages\n\ngo 1.27\n\nrequire example.com/greet v1.0.0\n",
        ),
        (
            "server/packages/go.sum",
            "example.com/greet v1.0.0 h1:first=\n",
        ),
        (
            "server/packages/vendor/modules.txt",
            "# example.com/greet v1.0.0\n## explicit; go 1.22\nexample.com/greet\n",
        ),
        (
            "server/packages/vendor/example.com/greet/greet.go",
            "package greet\n\nfunc Hello() string { return \"hello\" }\n",
        ),
    ];

    fn create() -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, text) in Self::FILES {
            Self::write(&root, relative, text);
        }
        root
    }

    fn write(root: &tempfile::TempDir, relative: &str, text: &str) {
        let path = root.path().join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(path, text).expect("file");
    }

    fn fingerprint(root: &tempfile::TempDir) -> String {
        GoNativeModule::read(root.path())
            .expect("native module")
            .fingerprint()
    }
}

#[test]
fn the_build_key_changes_when_go_sum_or_the_vendor_folder_changes() {
    let root = NativeProject::create();
    let first = NativeProject::fingerprint(&root);
    let edits = [
        (
            "server/packages/go.sum",
            "example.com/greet v1.0.0 h1:second=\n",
        ),
        (
            "server/packages/vendor/modules.txt",
            "# example.com/greet v1.0.0\n## explicit; go 1.23\nexample.com/greet\n",
        ),
        (
            "server/packages/vendor/example.com/greet/greet.go",
            "package greet\n\nfunc Hello() string { return \"hi\" }\n",
        ),
    ];

    let mut seen = vec![first];
    for (relative, text) in edits {
        NativeProject::write(&root, relative, text);
        let changed = NativeProject::fingerprint(&root);
        assert!(
            !seen.contains(&changed),
            "{relative} did not change the key"
        );
        seen.push(changed);
    }
}

#[test]
fn a_native_module_hosts_the_handler_program_in_its_reserved_folder() {
    let root = NativeProject::create();
    let native = GoNativeModule::read(root.path()).expect("native module");

    let runtime = native.runtime("import (\n\t\"yonhandler/yoninvocation\"\n)\n");

    assert_eq!(native.program_folder(), "yonhandler");
    assert_eq!(
        runtime,
        "import (\n\t\"example.com/shop/packages/yonhandler/yoninvocation\"\n)\n"
    );
}

#[test]
fn without_a_native_module_the_handler_keeps_its_own_module() {
    let root = tempfile::tempdir().expect("project");
    let native = GoNativeModule::read(root.path()).expect("no native module");
    let workspace = tempfile::tempdir().expect("workspace");

    native
        .write(workspace.path(), "server/controllers/ItemController.go")
        .expect("written");

    assert_eq!(native.program_folder(), "");
    let module = std::fs::read_to_string(workspace.path().join("go.mod")).expect("go.mod");
    assert!(module.starts_with("module yonhandler\n"), "{module}");
    assert!(!Path::new(&workspace.path().join("vendor")).exists());
}
