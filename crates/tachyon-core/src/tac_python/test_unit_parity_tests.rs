use super::runtime_assets::RuntimeAssets;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

struct PyodideTestParity;

impl PyodideTestParity {
    const UNIT: &'static str = "client/tests/PythonGreetingServiceTest.py";
    const CHILD: &'static str =
        "tac_python::test_unit_parity_tests::pyodide_child_prints_the_scaffold_test_report";
    const REPORT: &'static str = "pyodide-test-report ";
    const FILES: [(&'static str, &'static str); 3] = [
        (
            "client/services/PythonGreetingService.py",
            include_str!("../scaffold/templates/tac/py/services/PythonGreetingService.py.tmpl"),
        ),
        (
            "client/tests/PythonGreetingTestSetup.py",
            include_str!("../scaffold/templates/tac/py/tests/PythonGreetingTestSetup.py.tmpl"),
        ),
        (
            Self::UNIT,
            include_str!("../scaffold/templates/tac/py/tests/PythonGreetingServiceTest.py.tmpl"),
        ),
    ];

    fn required(name: &str) -> PathBuf {
        let value = std::env::var_os(name)
            .unwrap_or_else(|| panic!("{name} must name the explicit local fixture"));
        PathBuf::from(value)
    }

    fn verified_core() {
        let core = Self::required("TAC_PYTHON_CORE");
        RuntimeAssets::read(&core)
            .unwrap_or_else(|error| panic!("pinned Pyodide core assets: {error:?}"));
    }

    fn project() -> tempfile::TempDir {
        let project = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
        for (relative, source) in Self::FILES {
            let path = project.path().join(relative);
            let parent = path.parent().unwrap_or_else(|| panic!("source parent"));
            std::fs::create_dir_all(parent).unwrap_or_else(|error| panic!("folder: {error}"));
            std::fs::write(&path, source).unwrap_or_else(|error| panic!("source: {error}"));
        }
        project
    }

    fn launcher(directory: &Path) -> PathBuf {
        let path = directory.join("pyodide_test_unit.mjs");
        std::fs::write(&path, include_str!("pyodide_test_unit.mjs"))
            .unwrap_or_else(|error| panic!("launcher: {error}"));
        let executable = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&path, executable)
            .unwrap_or_else(|error| panic!("launcher mode: {error}"));
        path
    }

    fn native(project: &Path) -> String {
        assert!(
            std::env::var_os("YON_PYTHON_RUNTIME").is_none(),
            "the CPython side must use the default ty test interpreter"
        );
        let report = crate::client_tests::run(project, Path::new(Self::UNIT))
            .unwrap_or_else(|error| panic!("CPython test unit: {error:?}"));
        assert!(!report.cases.is_empty(), "the unit must run a case");
        assert!(report.cases.iter().all(|case| case.passed), "{report:?}");
        format!("{report:?}")
    }

    async fn pyodide(project: &Path, scratch: &Path) -> String {
        let executable =
            std::env::current_exe().unwrap_or_else(|error| panic!("test binary: {error}"));
        let mut command = tokio::process::Command::new(executable);
        command
            .args([Self::CHILD, "--exact", "--ignored", "--nocapture"])
            .arg("--test-threads=1")
            .env("YON_PYTHON_RUNTIME", Self::launcher(scratch))
            .env("TAC_PYTHON_UNIT_PROJECT", project)
            .env("TAC_PYTHON_UNIT_RECEIPT", scratch.join("receipt.json"));
        let output = crate::external_command::run(&mut command, Duration::from_mins(2), 1_048_576)
            .await
            .unwrap_or_else(|error| panic!("supervised Pyodide test unit: {error}"));
        let printed = String::from_utf8_lossy(&output.stdout).into_owned();
        assert!(
            output.status.success(),
            "Pyodide test unit: {printed}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Self::printed_report(&printed)
    }

    fn printed_report(printed: &str) -> String {
        let (_, rest) = printed
            .split_once(Self::REPORT)
            .unwrap_or_else(|| panic!("no Pyodide report in: {printed}"));
        rest.lines().next().unwrap_or_default().to_owned()
    }

    fn require_pyodide_receipt(scratch: &Path) {
        let text = std::fs::read_to_string(scratch.join("receipt.json"))
            .unwrap_or_else(|error| panic!("Pyodide receipt: {error}"));
        let receipt: serde_json::Value =
            serde_json::from_str(&text).unwrap_or_else(|error| panic!("receipt JSON: {error}"));
        assert_eq!(receipt["platform"], "emscripten");
        assert_eq!(receipt["pyodide"], RuntimeAssets::VERSION);
    }
}

#[tokio::test]
#[ignore = "requires the pinned five-file core; set TAC_PYTHON_CORE to the verified local directory and put Node 24 on PATH"]
async fn the_scaffold_python_test_unit_reports_the_same_cases_in_cpython_and_pinned_pyodide() {
    PyodideTestParity::verified_core();
    let project = PyodideTestParity::project();
    let scratch = tempfile::tempdir().unwrap_or_else(|error| panic!("scratch: {error}"));
    let native = PyodideTestParity::native(project.path());
    let pyodide = PyodideTestParity::pyodide(project.path(), scratch.path()).await;
    PyodideTestParity::require_pyodide_receipt(scratch.path());
    assert_eq!(pyodide, native);
}

#[test]
#[ignore = "runs only as the child of the Pyodide parity test, which sets YON_PYTHON_RUNTIME"]
fn pyodide_child_prints_the_scaffold_test_report() {
    PyodideTestParity::required("YON_PYTHON_RUNTIME");
    let project = PyodideTestParity::required("TAC_PYTHON_UNIT_PROJECT");
    let report = crate::client_tests::run(&project, Path::new(PyodideTestParity::UNIT))
        .unwrap_or_else(|error| panic!("Pyodide test unit: {error:?}"));
    println!("{}{report:?}", PyodideTestParity::REPORT);
}
