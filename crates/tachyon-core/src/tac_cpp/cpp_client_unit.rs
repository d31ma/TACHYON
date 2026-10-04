use super::{
    CppAttribute, CppFailure, CppFile, CppFolder, CppLowering, CppMarkerCheck, CppMemberKind,
    CppProgram, CppTools,
};
use crate::Failure;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

pub(crate) struct CppClientUnit {
    name: String,
    setup: String,
    cases: Vec<String>,
    lowered: String,
    path: PathBuf,
}

impl CppClientUnit {
    const TESTS: &'static str = "client/tests";
    const CASE_PREFIX: &'static str = "test";
    const SOURCE: &'static str = "unit.cpp";
    const BINARY: &'static str = "unit";
    const FLAGS: [&'static str; 3] = ["-std=c++20", "-O0", "-fno-color-diagnostics"];
    const FAILED: u16 = 2021;
    const NO_TEST: &'static str =
        "A C++ test unit declares one class marked [[Test(<Name>TestSetup)]]";
    const NO_SETUP: &'static str = "[[Test]] names its TestSetup class first, for example [[Test(TallyTestSetup)]], and client/tests/TallyTestSetup.hpp declares class [[TestSetup]] TallyTestSetup";

    pub(crate) fn run(project: &Path, relative: &Path, source: &str) -> Result<String, Failure> {
        let unit = Self::read(relative, source)?;
        let setup = Self::setup(project, &unit)?;
        let program = CppProgram::every_service(project)?;
        let located = CppProgram::located(&unit.path, &unit.lowered);
        let units = format!("{setup}{located}{}", unit.main());
        Self::compiled_and_run(relative, &program.test_source(&units))
    }

    fn read(relative: &Path, source: &str) -> Result<Self, Failure> {
        let file = CppFile::read(relative, source)?;
        CppMarkerCheck::check(relative, &file.class)?;
        let marker = file
            .class
            .markers
            .iter()
            .find(|marker| marker.name == "Test")
            .ok_or_else(|| CppFailure::unreadable(relative, file.class.name_at, Self::NO_TEST))?;
        let setup = Self::setup_name(relative, marker)?;
        let cases = file
            .class
            .members
            .iter()
            .filter(|member| member.is_public_instance(CppMemberKind::Method))
            .filter(|member| {
                [
                    member.name.starts_with(Self::CASE_PREFIX),
                    member.parameters.is_empty(),
                ] == [true, true]
            })
            .map(|member| member.name.clone())
            .collect();
        Ok(Self {
            name: file.class.name.clone(),
            setup,
            cases,
            lowered: CppLowering::model(&file, source),
            path: relative.to_path_buf(),
        })
    }

    fn setup_name(relative: &Path, marker: &CppAttribute) -> Result<String, Failure> {
        marker
            .arguments
            .split(',')
            .next()
            .map(str::trim)
            .filter(|name| [name.is_empty(), name.starts_with('{')] == [false, false])
            .map(str::to_owned)
            .ok_or_else(|| CppFailure::unreadable(relative, marker.at, Self::NO_SETUP))
    }

    fn setup(project: &Path, unit: &Self) -> Result<String, Failure> {
        let path = project
            .join(Self::TESTS)
            .join(format!("{}.hpp", unit.setup));
        let (relative, source) = CppFolder::read(project, &path)
            .map_err(|_| CppFailure::unreadable(&unit.path, 0, Self::NO_SETUP))?;
        let file = CppFile::read(&relative, &source)?;
        CppMarkerCheck::check(&relative, &file.class)?;
        let marked = file
            .class
            .markers
            .iter()
            .any(|marker| marker.name == "TestSetup");
        let valid = [marked, file.class.name == unit.setup] == [true, true];
        valid
            .then_some(())
            .ok_or_else(|| CppFailure::unreadable(&relative, file.class.name_at, Self::NO_SETUP))?;
        let lowered = CppLowering::model(&file, &source);
        Ok(CppProgram::located(&relative, &lowered))
    }

    fn main(&self) -> String {
        let mut cases = String::new();
        for case in &self.cases {
            let _ = writeln!(
                cases,
                "    runner.run(\"{case}\", [] {{ {} unit; unit.{case}(); }});",
                self.name
            );
        }
        format!(
            "#line 1 \"tachyon-test-main.cpp\"\nint main() {{\n    tachyon::TacTestRunner runner(\"{}\");\n{cases}    std::puts(runner.text().c_str());\n    return 0;\n}}\n",
            self.name
        )
    }

    fn compiled_and_run(relative: &Path, text: &str) -> Result<String, Failure> {
        let staged = tempfile::Builder::new()
            .prefix("tachyon-cpp-unit-")
            .tempdir()
            .map_err(|error| Self::failure(relative, &error.to_string()))?;
        std::fs::write(staged.path().join(Self::SOURCE), text)
            .map_err(|error| Self::failure(relative, &error.to_string()))?;
        let compiled = Command::new(CppTools::test_compiler())
            .args(Self::FLAGS)
            .args([Self::SOURCE, "-o", Self::BINARY])
            .current_dir(staged.path())
            .output()
            .map_err(|error| {
                Self::failure(
                    relative,
                    &format!("cannot start the C++ compiler: {error}. Run ty doctor"),
                )
            })?;
        Self::succeeded(relative, &compiled)?;
        let ran = Command::new(staged.path().join(Self::BINARY))
            .env("TAC_TEST_UNIT", "1")
            .output()
            .map_err(|error| Self::failure(relative, &error.to_string()))?;
        Self::succeeded(relative, &ran)?;
        Ok(String::from_utf8_lossy(&ran.stdout).into_owned())
    }

    fn succeeded(relative: &Path, output: &Output) -> Result<(), Failure> {
        output
            .status
            .success()
            .then_some(())
            .ok_or_else(|| Self::failure(relative, &String::from_utf8_lossy(&output.stderr)))
    }

    fn failure(relative: &Path, detail: &str) -> Failure {
        let message = format!("The C++ test unit did not run: {}", detail.trim());
        CppFailure::coded(Self::FAILED, relative, 0, &message)
    }
}
