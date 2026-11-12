#![allow(clippy::expect_used)]

use crate::support::one_test_per_case::one_test_per_case;
use crate::support::program_probe::ProgramProbe;
use crate::support::published_files::PublishedFiles;
use crate::support::toolchain::Toolchain;
use std::fs;
use std::path::Path;
use tachyon_core::{BuildOptions, WebCompiler};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MenuLanguage {
    extension: &'static str,
    program: &'static str,
    flag: &'static str,
}

impl MenuLanguage {
    const SECTION: &'static str = "docs/templates/keys-modifiers-and-outside-clicks";
    const SERVICE_STEM: &'static str = "client/services/MenuService.";
    const PYTHON: &'static str = "py";
    const SWIFT: &'static str = "swift";
    const SWIFT_WASM_SDK: &'static str = "TAC_SWIFT_WASM_SDK";
    const SWIFT_SYSTEM_ROOT: &'static str = "WASI.sdk";
    const PYODIDE_WASM: &'static str = "python-web/314.0.7/pyodide.asm.wasm";
    const PLAN_MEMBERS: [&'static str; 7] = [
        r#""eventType":"keydown""#,
        r#""listen":"document""#,
        r#""key":"Escape""#,
        r#""modifierKeys":[]"#,
        r#""prevent":true"#,
        r#""outside":true"#,
        r#""handler":"menu.toggle""#,
    ];
    const ALL: [Self; 14] = [
        Self::new("js", "node", "--version"),
        Self::new("ts", "node", "--version"),
        Self::new(Self::PYTHON, "python3", "--version"),
        Self::new("java", "javac", "--version"),
        Self::new("kt", "kotlinc", "-version"),
        Self::new("cs", "dotnet", "--version"),
        Self::new("fs", "dotnet", "--version"),
        Self::new("rs", "rustc", "--version"),
        Self::new("dart", "dart", "--version"),
        Self::new("go", "go", "version"),
        Self::new(Self::SWIFT, "swiftc", "--version"),
        Self::new("rb", "ruby", "--version"),
        Self::new("lua", "lua", "-v"),
        Self::new("hpp", "clang++", "--version"),
    ];

    const fn new(extension: &'static str, program: &'static str, flag: &'static str) -> Self {
        Self {
            extension,
            program,
            flag,
        }
    }

    fn available(&self) -> bool {
        self.program_found() && self.browser_runtime_found()
    }

    fn browser_runtime_found(&self) -> bool {
        match self.extension {
            Self::PYTHON => Self::pyodide_found(),
            Self::SWIFT => Self::swift_sdk_found(),
            _ => true,
        }
    }

    fn swift_sdk_found() -> bool {
        let found = std::env::var_os(Self::SWIFT_WASM_SDK)
            .is_some_and(|sdk| Path::new(&sdk).join(Self::SWIFT_SYSTEM_ROOT).is_dir());
        if !found {
            eprintln!(
                "skipping .swift: {} does not name a Swift SDK for WebAssembly",
                Self::SWIFT_WASM_SDK
            );
        }
        found
    }

    fn pyodide_found() -> bool {
        let found = tachyon_core::cache::root()
            .join(Self::PYODIDE_WASM)
            .is_file();
        assert!(
            found || std::env::var_os("CI").is_none(),
            "CI installs the pinned Pyodide, so the Python menu must build there"
        );
        if !found {
            eprintln!("skipping .py: the pinned Pyodide runtime is not in the Tachyon cache");
        }
        found
    }

    fn program_found(&self) -> bool {
        let found =
            Toolchain::checked(self.program, ProgramProbe::answers(self.program, self.flag));
        if !found {
            eprintln!(
                "skipping .{}: {} is unavailable",
                self.extension, self.program
            );
        }
        found
    }

    fn belongs(&self, relative: &str) -> bool {
        !relative.starts_with(Self::SERVICE_STEM)
            || relative == format!("{}{}", Self::SERVICE_STEM, self.extension)
    }

    fn write_project(&self, root: &Path) {
        let files = PublishedFiles::read(&PublishedFiles::folder(Self::SECTION), "client/");
        for (relative, code) in files
            .into_iter()
            .filter(|(relative, _)| self.belongs(relative))
        {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            fs::write(path, code).expect("source");
        }
    }

    fn build_plan(&self) -> String {
        let project = tempfile::tempdir().expect("project");
        self.write_project(project.path());
        let options = BuildOptions {
            output_directory: project.path().join("dist"),
            ..BuildOptions::default()
        };
        WebCompiler::build(project.path(), &options)
            .unwrap_or_else(|failure| panic!(".{}: {failure}", self.extension));
        fs::read_to_string(project.path().join("dist/index.html")).expect("page")
    }

    fn check(self) {
        if !self.available() {
            return;
        }
        let plan = self.build_plan();
        for member in Self::PLAN_MEMBERS {
            assert!(
                plan.contains(member),
                ".{}: {member} in {plan}",
                self.extension
            );
        }
        assert_eq!(
            plan.matches(r#""handler":"menu.close""#).count(),
            3,
            ".{}: {plan}",
            self.extension
        );
    }
}

fn check_language(language: MenuLanguage) {
    language.check();
}

one_test_per_case!(
    menu_example_builds_with_event_modifiers,
    check_language,
    all MenuLanguage::ALL,
    [
        javascript => MenuLanguage::ALL[0],
        typescript => MenuLanguage::ALL[1],
        python => MenuLanguage::ALL[2],
        java => MenuLanguage::ALL[3],
        kotlin => MenuLanguage::ALL[4],
        csharp => MenuLanguage::ALL[5],
        fsharp => MenuLanguage::ALL[6],
        rust => MenuLanguage::ALL[7],
        dart => MenuLanguage::ALL[8],
        go => MenuLanguage::ALL[9],
        swift => MenuLanguage::ALL[10],
        ruby => MenuLanguage::ALL[11],
        lua => MenuLanguage::ALL[12],
        cpp => MenuLanguage::ALL[13],
    ]
);
