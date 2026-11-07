#![allow(clippy::expect_used)]

use crate::support::one_test_per_case::one_test_per_case;
use crate::support::published_files::PublishedFiles;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct TacLanguage {
    extension: &'static str,
    program: &'static str,
    flag: &'static str,
    cases: usize,
}

impl TacLanguage {
    const JAVASCRIPT: Self = Self::new("js", "node", "--version", 2);
    const TYPESCRIPT: Self = Self::new("ts", "node", "--version", 2);
    const PYTHON: Self = Self::new("py", "python3", "--version", 2);
    const JAVA: Self = Self::new("java", "javac", "--version", 2);
    const KOTLIN: Self = Self::new("kt", "kotlinc", "-version", 2);
    const CSHARP: Self = Self::new("cs", "dotnet", "--version", 3);
    const FSHARP: Self = Self::new("fs", "dotnet", "--version", 3);
    const RUST: Self = Self::new("rs", "rustc", "--version", 2);
    const DART: Self = Self::new("dart", "dart", "--version", 2);
    const GO: Self = Self::new("go", "go", "version", 2);
    const SWIFT: Self = Self::new("swift", "swiftc", "--version", 2);
    const RUBY: Self = Self::new("rb", "ruby", "--version", 2);
    const LUA: Self = Self::new("lua", "lua", "-v", 2);
    const ALL: [Self; 13] = [
        Self::JAVASCRIPT,
        Self::TYPESCRIPT,
        Self::PYTHON,
        Self::JAVA,
        Self::KOTLIN,
        Self::CSHARP,
        Self::FSHARP,
        Self::RUST,
        Self::DART,
        Self::GO,
        Self::SWIFT,
        Self::RUBY,
        Self::LUA,
    ];
    const UNIX_ONLY_IN_CI: [&'static str; 1] = ["swiftc"];

    const fn new(
        extension: &'static str,
        program: &'static str,
        flag: &'static str,
        cases: usize,
    ) -> Self {
        Self {
            extension,
            program,
            flag,
            cases,
        }
    }

    fn available(&self) -> bool {
        let found = crate::support::program_probe::ProgramProbe::answers(self.program, self.flag);
        let required = [
            std::env::var_os("CI").is_some(),
            cfg!(unix) || !Self::UNIX_ONLY_IN_CI.contains(&self.program),
        ] == [true, true];
        assert!(found || !required, "{} is missing in CI", self.program);
        if !found {
            eprintln!(
                "skipping .{}: {} is unavailable",
                self.extension, self.program
            );
        }
        found
    }

    fn run_first_app_tabs(&self) {
        if !self.available() {
            return;
        }
        let project = tempfile::tempdir().expect("project");
        Step::ADD_A_SERVICE.write(project.path(), self);
        Step::CALL_THE_SERVER.write(project.path(), self);
        Step::ADD_TESTS.write(project.path(), self);
        let unit = format!("client/tests/CounterServiceTest.{}", self.extension);
        let report = tachyon_core::client_tests::run(project.path(), Path::new(&unit))
            .unwrap_or_else(|failure| panic!("{}: {failure}", self.extension));
        assert!(report.failure.is_none(), "{}: {report:?}", self.extension);
        assert_eq!(
            report.cases.len(),
            self.cases,
            "{}: {report:?}",
            self.extension
        );
        let passed: Vec<bool> = report.cases.iter().map(|case| case.passed).collect();
        assert_eq!(
            passed,
            vec![true; self.cases],
            "{}: {report:?}",
            self.extension
        );
    }
}

struct Step {
    folder: &'static str,
}

impl Step {
    const ADD_A_SERVICE: Self = Self {
        folder: "add-a-service",
    };
    const CALL_THE_SERVER: Self = Self {
        folder: "call-the-server",
    };
    const ADD_TESTS: Self = Self {
        folder: "add-tests",
    };

    fn source(&self) -> PathBuf {
        PublishedFiles::folder("docs/first-app").join(self.folder)
    }

    fn write(&self, root: &Path, language: &TacLanguage) {
        let found = PublishedFiles::read(&self.source(), "client/");
        let suffix = format!(".{}", language.extension);
        let own_code: String = found
            .iter()
            .filter(|(relative, _)| relative.ends_with(&suffix))
            .map(|(_, code)| code.as_str())
            .collect();
        let chosen = found.into_iter().filter(|(relative, _)| {
            let realm_path = relative.trim_start_matches("client");
            relative.ends_with(&suffix)
                || relative.ends_with("tac.html")
                || own_code.contains(realm_path)
        });
        for (relative, code) in chosen {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            fs::write(path, code).expect("source");
        }
    }
}

fn build_and_test(language: TacLanguage) {
    language.run_first_app_tabs();
}

one_test_per_case!(
    the_first_app_tabs,
    build_and_test,
    all TacLanguage::ALL,
    [
        the_first_app_javascript_tabs_build_and_pass_their_tests => TacLanguage::JAVASCRIPT,
        the_first_app_typescript_tabs_build_and_pass_their_tests => TacLanguage::TYPESCRIPT,
        the_first_app_python_tabs_build_and_pass_their_tests => TacLanguage::PYTHON,
        the_first_app_java_tabs_build_and_pass_their_tests => TacLanguage::JAVA,
        the_first_app_kotlin_tabs_build_and_pass_their_tests => TacLanguage::KOTLIN,
        the_first_app_csharp_tabs_build_and_pass_their_tests => TacLanguage::CSHARP,
        the_first_app_fsharp_tabs_build_and_pass_their_tests => TacLanguage::FSHARP,
        the_first_app_rust_tabs_build_and_pass_their_tests => TacLanguage::RUST,
        the_first_app_dart_tabs_build_and_pass_their_tests => TacLanguage::DART,
        the_first_app_go_tabs_build_and_pass_their_tests => TacLanguage::GO,
        the_first_app_swift_tabs_build_and_pass_their_tests => TacLanguage::SWIFT,
        the_first_app_ruby_tabs_build_and_pass_their_tests => TacLanguage::RUBY,
        the_first_app_lua_tabs_build_and_pass_their_tests => TacLanguage::LUA,
    ]
);
