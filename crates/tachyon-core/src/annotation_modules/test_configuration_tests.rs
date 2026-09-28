#![allow(clippy::expect_used)]

use super::*;

fn write(root: &Path, path: &str, source: &str) {
    let path = root.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    std::fs::write(path, source).expect("source");
}

fn javascript_unit() -> &'static str {
    "@Test(ShellTestSetup)\nexport default class ShellServiceTest {}\n"
}

fn compose(root: &Path) -> Result<Option<String>, Failure> {
    assemble(
        root,
        Path::new("client/tests/ShellServiceTest.js"),
        javascript_unit(),
        "tac",
    )
}

fn assert_fails_with(result: Result<Option<String>, Failure>, code: u16, message: &str) {
    let failure = result.expect_err(message);
    let diagnostic = &failure.diagnostics()[0];
    assert_eq!(diagnostic.code.number(), code, "{failure}");
    assert!(diagnostic.message.contains(message), "{failure}");
}

fn project() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    write(
        root.path(),
        "client/tests/ShellServiceTest.js",
        javascript_unit(),
    );
    root
}

#[test]
fn a_unit_composes_only_its_named_sibling_test_configuration() {
    let root = project();
    write(
        root.path(),
        "client/tests/ShellTestSetup.js",
        "@TestSetup()\nclass ShellTestSetup { static configure() { return {}; } }\n",
    );
    write(
        root.path(),
        "client/tests/OtherTestConfiguration.js",
        "INVALID UNSELECTED SOURCE",
    );
    let output = compose(root.path())
        .expect("compose")
        .expect("configuration");
    assert!(output.contains("class ShellTestSetup"));
    assert!(!output.contains("INVALID UNSELECTED SOURCE"));
    assert!(output.find("class ShellTestSetup") < output.find("class ShellServiceTest"));
}

#[test]
fn missing_and_mismatched_test_configurations_fail_before_lowering() {
    let root = project();
    assert_fails_with(
        compose(root.path()),
        1310,
        "The named test configuration is missing",
    );
    let not_the_owned_class = "TestSetup must annotate the owned top-level configuration class";
    for (source, code, message) in [
        (
            "@TestSetup()\nclass OtherTestConfiguration {}\n",
            crate::annotation_layout::FileNameCheck::CODE,
            "does not match the class 'OtherTestConfiguration'",
        ),
        ("class ShellTestSetup {}\n", 1310, "must carry TestSetup"),
        (
            "@TestSetup()\nclass ShellTestSetup {}\nclass Helper {}\n",
            1310,
            "holds one class and its imports; found another class 'Helper'",
        ),
        (
            "@TestSetup(1)\nclass ShellTestSetup {}\n",
            1308,
            "TestSetup accepts no arguments",
        ),
        (
            "@TestSetup()\n@TestSetup()\nclass ShellTestSetup {}\n",
            1308,
            "@TestSetup is written twice on one class",
        ),
        (
            "class ShellTestSetup {\n@TestSetup()\nconfigure() {}\n}\n",
            1308,
            "@TestSetup is on a method, but it belongs on a class",
        ),
        (
            "class ShellTestSetup {\nmethod() {\n@TestSetup()\nclass Nested {}\n}\n}\n",
            1308,
            not_the_owned_class,
        ),
    ] {
        write(root.path(), "client/tests/ShellTestSetup.js", source);
        assert_fails_with(compose(root.path()), code, message);
    }
}

#[test]
fn a_test_configuration_file_name_matches_with_exact_case() {
    let root = project();
    write(
        root.path(),
        "client/tests/shelltestconfiguration.js",
        "@TestSetup()\nclass ShellTestSetup {}\n",
    );
    assert_fails_with(
        compose(root.path()),
        1310,
        "The named test configuration is missing",
    );
}

#[test]
fn test_configuration_names_are_not_paths() {
    let root = project();
    for (name, code, message) in [
        ("../ShellTestSetup", 1310, "runs code when the file loads"),
        (
            "nested/ShellTestSetup",
            1310,
            "runs code when the file loads",
        ),
        (
            "ShellTestSetup.js",
            1308,
            "Test names one Setup class in this source",
        ),
        (
            "\"ShellTestSetup\"",
            1308,
            "Test names one Setup class in this source",
        ),
    ] {
        let source = format!("@Test({name})\nclass ShellServiceTest {{}}\n");
        assert_fails_with(
            assemble(
                root.path(),
                Path::new("client/tests/ShellServiceTest.js"),
                &source,
                "tac",
            ),
            code,
            message,
        );
    }
}

#[test]
fn test_configurations_share_the_annotation_source_budget() {
    let root = project();
    let oversized = format!(
        "//{}\n@TestSetup()\nclass ShellTestSetup {{}}\n",
        "x".repeat(Module::MAX_SOURCE)
    );
    write(root.path(), "client/tests/ShellTestSetup.js", &oversized);
    assert_fails_with(
        compose(root.path()),
        2101,
        "Module files must be bounded regular files",
    );
}

#[test]
fn behavior_only_files_are_composed_without_managed_roles() {
    let root = tempfile::tempdir().expect("project");
    let entry = Path::new("server/controllers/MoneyController.java");
    let source = "@RequestRoute(\"/money\")\n@Public\nclass MoneyController {\n    @GetRoute\n    Object get() { return null; }\n}\n";
    write(
        root.path(),
        "server/controllers/MoneyController.java",
        source,
    );
    write(
        root.path(),
        "server/helpers/Catalog.java",
        "@CacheSettings({\"name\":\"catalog\",\"ttlMs\":1000,\"maxBytes\":1000,\"maxEntries\":10})\nclass Catalog {}\n",
    );
    write(root.path(), "server/helpers/Plain.java", "class Plain {}\n");
    let output = assemble(root.path(), entry, source, "yon")
        .expect("composition")
        .expect("behavior module");
    assert!(output.contains("class Catalog"));
    assert!(!output.contains("@Helper"));
    assert!(!output.contains("class Plain"));
    write(
        root.path(),
        "server/helpers/Catalog.java",
        "@CacheSettings({\"name\":\"catalog\",\"ttlMs\":1000,\"maxBytes\":1000,\"maxEntries\":10})\nclass Catalog {}\nclass Hidden {}\n",
    );
    assert_fails_with(
        assemble(root.path(), entry, source, "yon"),
        1310,
        "found another class 'Hidden'",
    );
}

#[test]
fn a_named_production_configuration_remains_supported() {
    let root = project();
    write(
        root.path(),
        "client/setups/ShellTestSetup.js",
        "@Setup()\nclass ShellTestSetup { static configure() { return {}; } }\n",
    );
    let output = compose(root.path())
        .expect("compose")
        .expect("configuration");
    assert_eq!(output.matches("class ShellTestSetup").count(), 1);
    write(
        root.path(),
        "client/tests/ShellTestSetup.js",
        "@TestSetup()\nclass ShellTestSetup {}\n",
    );
    assert_fails_with(
        compose(root.path()),
        2101,
        "Annotation modules declare a duplicate type name.",
    );
}

#[test]
fn a_production_configuration_fallback_cannot_name_another_kind_of_type() {
    for (name, path, source) in [
        (
            "ShellService",
            "client/services/ShellService.js",
            "@Service()\nclass ShellService {}\n",
        ),
        (
            "ShellTestSetup",
            "client/setups/ShellTestSetup.js",
            "@Lifetime(\"transient\")\nclass ShellTestSetup {}\n",
        ),
    ] {
        let root = project();
        write(root.path(), path, source);
        let unit = format!("@Test({name})\nclass ShellServiceTest {{}}\n");
        let error = assemble(
            root.path(),
            Path::new("client/tests/ShellServiceTest.js"),
            &unit,
            "tac",
        )
        .expect_err("not a production Setup");
        assert_eq!(error.diagnostics()[0].code.number(), 1310, "{error}");
    }
}

#[test]
fn test_configuration_never_enters_the_production_graph() {
    let root = project();
    write(
        root.path(),
        "client/tests/ShellTestSetup.js",
        "@TestSetup()\nclass ShellTestSetup {}\n",
    );
    let entry = Path::new("client/services/ShellService.js");
    let source = "@Service()\nexport default class ShellService {}\n";
    write(root.path(), entry.to_str().expect("path"), source);
    assert!(
        assemble(root.path(), entry, source, "tac")
            .expect("production")
            .is_none()
    );
}

#[test]
fn misplaced_test_markers_cannot_enter_a_production_role_graph() {
    for source in [
        "@TestSetup()\nclass LeakedTestSetup {}\n",
        "@Test(LeakedTestSetup)\nclass LeakedTestSetup {}\n",
        "class LeakedTestSetup {\n@TestSetup()\nconfigure() {}\n}\n",
        "class LeakedTestSetup {\n@Test(LeakedTestSetup)\nrun() {}\n}\n",
    ] {
        for realm in ["client", "server"] {
            let root = tempfile::tempdir().expect("project");
            let (entry, entry_source) = if realm == "client" {
                (
                    "client/services/ShellService.js",
                    "@Service()\nclass ShellService {}\n",
                )
            } else {
                (
                    "server/controllers/MoneyController.js",
                    "@RequestRoute(\"/money\")\n@Public\nclass MoneyController {\n  @GetRoute\n  get() {}\n}\n",
                )
            };
            write(root.path(), entry, entry_source);
            let misplaced = format!("{realm}/setups/LeakedTestSetup.js");
            write(root.path(), &misplaced, source);
            let environment = if realm == "client" { "tac" } else { "yon" };
            let failure = assemble(root.path(), Path::new(entry), entry_source, environment)
                .expect_err(&format!("{realm} test-only marker outside tests: {source}"));
            let diagnostic = &failure.diagnostics()[0];
            assert_eq!(diagnostic.code.number(), 1310, "{failure}");
            assert!(
                diagnostic
                    .help
                    .as_deref()
                    .expect("placement guidance")
                    .contains("tests/")
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn a_test_configuration_must_not_be_a_symlink() {
    let root = project();
    let outside = tempfile::tempdir().expect("outside");
    write(
        outside.path(),
        "ShellTestSetup.js",
        "@TestSetup()\nclass ShellTestSetup {}\n",
    );
    std::os::unix::fs::symlink(
        outside.path().join("ShellTestSetup.js"),
        root.path().join("client/tests/ShellTestSetup.js"),
    )
    .expect("symlink");
    assert_fails_with(compose(root.path()), 2101, "not symlinks");
}

#[test]
fn sibling_test_configurations_compose_in_every_realm_language() {
    for (extension, annotation, end, declaration) in [
        ("js", "@", "", "class"),
        ("ts", "@", "", "class"),
        ("py", "@", "", "class"),
        ("php", "#[", "]", "class"),
        ("java", "@", "", "class"),
        ("kt", "@", "", "class"),
        ("cs", "[", "]", "class"),
        ("rs", "#[", "]", "struct"),
        ("swift", "@", "", "class"),
        ("dart", "@", "", "class"),
    ] {
        let opening = if extension == "php" { "<?php\n" } else { "" };
        let body = if extension == "py" {
            ":\n    pass\n"
        } else {
            " {}\n"
        };
        let unit = format!(
            "{opening}{annotation}Test(ShellTestSetup){end}\n{declaration} ShellServiceTest{body}"
        );
        let configuration =
            format!("{opening}{annotation}TestSetup(){end}\n{declaration} ShellTestSetup{body}");
        for realm in ["client", "server"] {
            if (realm == "client" && matches!(extension, "py" | "php" | "java"))
                || (realm == "server" && extension == "swift")
            {
                continue;
            }
            let root = tempfile::tempdir().expect("project");
            let entry = format!("{realm}/tests/ShellServiceTest.{extension}");
            write(root.path(), &entry, &unit);
            write(
                root.path(),
                &format!("{realm}/tests/ShellTestSetup.{extension}"),
                &configuration,
            );
            let environment = if realm == "client" { "tac" } else { "yon" };
            let output = assemble(root.path(), Path::new(&entry), &unit, environment)
                .unwrap_or_else(|error| panic!("{realm}/{extension}: {error}"))
                .expect("configuration");
            assert!(output.contains("ShellTestSetup"), "{realm}/{extension}");
        }
    }
}
