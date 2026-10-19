#![allow(clippy::expect_used)]

use std::path::Path;

struct HttpVerbProject;

impl HttpVerbProject {
    const LANGUAGES: [(&'static str, &'static str); 12] = [
        ("js", "node"),
        ("ts", "node"),
        ("dart", "dart"),
        ("rs", "cargo"),
        ("cs", "dotnet"),
        ("kt", "kotlinc"),
        ("swift", "swiftc"),
        ("go", "go"),
        ("rb", "ruby"),
        ("py", "python3"),
        ("lua", "lua"),
        ("hpp", "clang++"),
    ];
    const VERBS: [&'static str; 3] = ["put", "patch", "delete"];

    const VERSION_FLAGS: [(&'static str, &'static str); 3] =
        [("kotlinc", "-version"), ("go", "version"), ("lua", "-v")];

    fn tool_available(program: &str) -> bool {
        let flag = Self::VERSION_FLAGS
            .iter()
            .find(|(name, _)| *name == program)
            .map_or("--version", |(_, flag)| flag);
        std::process::Command::new(program)
            .arg(flag)
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn copy(from: &Path, to: &Path) {
        for entry in std::fs::read_dir(from).expect("fixture folder").flatten() {
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                std::fs::create_dir_all(&target).expect("folder");
                Self::copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).expect("fixture file");
            }
        }
    }

    fn assert_unit_passes(extension: &str) {
        let root = tempfile::tempdir().expect("project");
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/tac-http-verbs")
            .join(extension);
        Self::copy(&fixture, root.path());
        let unit = format!("client/tests/OrderServiceTest.{extension}");
        let report = tachyon_core::client_tests::run(root.path(), Path::new(&unit))
            .unwrap_or_else(|error| panic!("{extension}: {error:?}"));
        let verbs: Vec<String> = report
            .cases
            .iter()
            .map(|case| case.name.to_lowercase().replace('_', ""))
            .collect();
        assert_eq!(verbs.len(), Self::VERBS.len(), "{extension}: {report:?}");
        for verb in Self::VERBS {
            assert!(
                verbs
                    .iter()
                    .any(|name| name.starts_with(&format!("test{verb}"))),
                "{extension} has no {verb} case: {report:?}"
            );
        }
        assert!(
            report.cases.iter().all(|case| case.passed),
            "{extension}: {report:?}"
        );
    }
}

#[test]
fn a_tac_http_client_sends_put_patch_delete_and_headers_in_every_language() {
    let mut ran = 0;
    for (extension, program) in HttpVerbProject::LANGUAGES {
        if !HttpVerbProject::tool_available(program) {
            eprintln!("skipping the {extension} HTTP verb unit: {program} is unavailable");
            continue;
        }
        HttpVerbProject::assert_unit_passes(extension);
        ran += 1;
    }
    assert!(ran > 0, "no Tac language toolchain is installed");
}
