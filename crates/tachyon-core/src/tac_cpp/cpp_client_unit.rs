use super::{CppFailure, CppProgram, CppTestUnit, CppTools};
use crate::Failure;
use std::fmt::Write as _;
use std::path::Path;
use std::process::{Command, Output};

pub(crate) struct CppClientUnit;

impl CppClientUnit {
    const SOURCE: &'static str = "unit.cpp";
    const BINARY: &'static str = "unit";
    const FLAGS: [&'static str; 3] = ["-std=c++20", "-O0", "-fno-color-diagnostics"];
    const FAILED: u16 = 2021;

    pub(crate) fn run(project: &Path, relative: &Path, source: &str) -> Result<String, Failure> {
        let unit = CppTestUnit::read(project, relative, source)?;
        let program = CppProgram::tested(project)?;
        unit.check_fake_types(|name| program.has_class(name))?;
        let units = format!("{}{}", unit.source(), Self::main(&unit));
        Self::compiled_and_run(relative, &program.test_source(&units))
    }

    fn main(unit: &CppTestUnit) -> String {
        let fresh = ["", "tachyon::TacFreshStart::next(); "][usize::from(unit.fresh)];
        let opened = format!("{}{fresh}", unit.opened());
        let mut cases = String::new();
        for case in &unit.cases {
            let _ = writeln!(
                cases,
                "    runner.run(\"{}\", [] {{ {opened}{} unit; tachyon::TacTestCase::attach(unit, answers.context()); tachyon::TacTestCase::finish([&unit] {{ return unit.{}; }}); }});",
                case.name, unit.name, case.call
            );
        }
        format!(
            "int main() {{\n    tachyon::TacTestRunner runner(\"{}\");\n{cases}    std::puts(runner.text().c_str());\n    return 0;\n}}\n",
            unit.name
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
