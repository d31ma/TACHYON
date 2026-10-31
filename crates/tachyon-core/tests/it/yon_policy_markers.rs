#![allow(clippy::expect_used)]

use crate::support::fixture_folder::FixtureFolder;
use crate::support::json_http::JsonHttp;
use crate::support::one_test_per_case::one_test_per_case;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
struct YonPolicyMarkers {
    extension: &'static str,
    program: &'static str,
    probe: &'static str,
}

impl YonPolicyMarkers {
    const ALL: [Self; 5] = [
        Self::new("java", "javac", "--version"),
        Self::new("groovy", "groovyc", "--version"),
        Self::new("kt", "kotlinc", "-version"),
        Self::new("cs", "dotnet", "--version"),
        Self::new("fs", "dotnet", "--version"),
    ];
    const NAMED: [&'static str; 4] = ["java", "groovy", "kt", "cs"];
    const ROLLED_BACK: [&'static str; 8] = [
        "begin",
        "find@t1",
        "count@t1",
        "commit:t1",
        "begin",
        "save@t2",
        "rollback:t2",
        "the ledger refused the entry",
    ];

    const fn new(extension: &'static str, program: &'static str, probe: &'static str) -> Self {
        Self {
            extension,
            program,
            probe,
        }
    }

    fn of(extension: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|language| language.extension == extension)
            .expect("a policy language")
    }

    fn available(self) -> bool {
        let found = crate::support::program_probe::ProgramProbe::answers(self.program, self.probe);
        if !crate::support::toolchain::Toolchain::checked(self.program, found) {
            eprintln!(
                "skipping .{}: {} is unavailable",
                self.extension, self.program
            );
        }
        found
    }

    fn fixture(self, policy: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/yon-policy")
            .join(policy)
            .join(self.extension)
    }

    async fn answer(self, policy: &str, route: &str) -> serde_json::Value {
        let project = tempfile::tempdir().expect("project");
        FixtureFolder::copy(&self.fixture(policy), project.path());
        let answers = JsonHttp::get_all(project.path(), &[route]).await;
        let (head, body) = answers[0]
            .split_once("\r\n\r\n")
            .unwrap_or_else(|| panic!("{}: {}", self.extension, answers[0]));
        assert!(
            head.starts_with("HTTP/1.1 200 OK"),
            "{}: {}",
            self.extension,
            answers[0]
        );
        serde_json::from_str(body)
            .unwrap_or_else(|_| panic!("{}: JSON answer: {}", self.extension, answers[0]))
    }
}

async fn rolls_back(extension: &str) {
    let language = YonPolicyMarkers::of(extension);
    if !language.available() {
        return;
    }
    let report = language.answer("all-or-nothing", "/ledger").await;
    assert_eq!(
        report["commands"],
        serde_json::json!(YonPolicyMarkers::ROLLED_BACK),
        "{extension}: {report}"
    );
}

async fn gives_the_named_service(extension: &str) {
    let language = YonPolicyMarkers::of(extension);
    if !language.available() {
        return;
    }
    let greeting = language.answer("named", "/greeting").await;
    assert_eq!(
        greeting,
        serde_json::json!({"named": "HELLO", "plain": "hello"}),
        "{extension}"
    );
}

one_test_per_case!(
    all_or_nothing_rolls_back_on_failure,
    async rolls_back,
    all YonPolicyMarkers::ALL.map(|language| language.extension),
    [
        java => "java",
        groovy => "groovy",
        kotlin => "kt",
        csharp => "cs",
        fsharp => "fs",
    ]
);

one_test_per_case!(
    named_needs_field_gets_the_named_service,
    async gives_the_named_service,
    all YonPolicyMarkers::NAMED,
    [
        java => "java",
        groovy => "groovy",
        kotlin => "kt",
        csharp => "cs",
    ]
);
