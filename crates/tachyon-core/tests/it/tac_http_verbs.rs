#![allow(clippy::expect_used)]

use crate::support::one_test_per_case::one_test_per_case;
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
    const POSTED: [&'static str; 1] = ["ts"];

    const VERSION_FLAGS: [(&'static str, &'static str); 3] =
        [("kotlinc", "-version"), ("go", "version"), ("lua", "-v")];

    fn tool_available(program: &str) -> bool {
        let flag = Self::VERSION_FLAGS
            .iter()
            .find(|(name, _)| *name == program)
            .map_or("--version", |(_, flag)| flag);
        crate::support::program_probe::ProgramProbe::answers(program, flag)
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
        let expected: Vec<&str> = Self::VERBS
            .into_iter()
            .chain(Self::POSTED.contains(&extension).then_some("post"))
            .collect();
        let verbs: Vec<String> = report
            .cases
            .iter()
            .map(|case| case.name.to_lowercase().replace('_', ""))
            .collect();
        assert_eq!(verbs.len(), expected.len(), "{extension}: {report:?}");
        for verb in expected {
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

fn sends_put_patch_delete_and_headers_in(wanted: &str) {
    let (extension, program) = HttpVerbProject::LANGUAGES
        .into_iter()
        .find(|(extension, _)| *extension == wanted)
        .unwrap_or_else(|| panic!("no HTTP verb language uses .{wanted}"));
    if !HttpVerbProject::tool_available(program) {
        eprintln!("skipping the {extension} HTTP verb unit: {program} is unavailable");
        return;
    }
    HttpVerbProject::assert_unit_passes(extension);
}

one_test_per_case!(
    a_tac_http_client_sends_put_patch_delete_and_headers_in_every_language,
    sends_put_patch_delete_and_headers_in,
    all HttpVerbProject::LANGUAGES.map(|(extension, _)| extension),
    [
        javascript => "js",
        typescript => "ts",
        dart => "dart",
        rust => "rs",
        csharp => "cs",
        kotlin => "kt",
        swift => "swift",
        go => "go",
        ruby => "rb",
        python => "py",
        lua => "lua",
        cpp => "hpp",
    ]
);

#[test]
fn at_least_one_tac_http_verb_language_toolchain_is_installed() {
    let installed = HttpVerbProject::LANGUAGES
        .into_iter()
        .any(|(_, program)| HttpVerbProject::tool_available(program));
    assert!(installed, "no Tac language toolchain is installed");
}
