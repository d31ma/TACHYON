use super::MigrationFix;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn project() -> TempDir {
    tempfile::tempdir().expect("project")
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("source");
}

fn names_in(root: &Path, folder: &str) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root.join(folder))
        .expect("folder")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    names
}

fn fix(root: &Path) -> String {
    let planned = MigrationFix::plan(root).expect("plan");
    let text = planned.to_text(false);
    planned.write(root).expect("write");
    text
}

#[test]
fn a_misnamed_class_file_is_renamed_after_its_class() {
    let project = project();
    let root = project.path();
    write(
        root,
        "server/models/Order.ts",
        "export interface Invoice {\n  id: string\n}\n",
    );
    write(
        root,
        "client/tests/ShellServiceTest.js",
        "class ShellChecks {}\n",
    );
    write(
        root,
        "server/models/Line.ts",
        "export interface Line {\n  id: string\n}\n",
    );
    fix(root);
    assert_eq!(names_in(root, "server/models"), ["Invoice.ts", "Line.ts"]);
    assert_eq!(names_in(root, "client/tests"), ["ShellChecks.js"]);
    let again = MigrationFix::plan(root).expect("plan");
    assert_eq!(again.move_count(), 0, "{}", again.to_text(false));
}

#[test]
fn a_name_that_differs_only_in_case_is_renamed() {
    let project = project();
    let root = project.path();
    write(
        root,
        "server/models/order.py",
        "from dataclasses import dataclass\n\n@dataclass\nclass Order:\n    id: str\n",
    );
    fix(root);
    assert_eq!(names_in(root, "server/models"), ["Order.py"]);
}

#[test]
fn a_file_that_others_name_keeps_its_name_and_its_class_is_renamed() {
    let project = project();
    let root = project.path();
    write(
        root,
        "server/delegates/ReportDelegate.rb",
        "class Report\n  def self.run\n  end\nend\n\nReport.run\n",
    );
    write(
        root,
        "server/controllers/ReportController.js",
        "const script = 'server/delegates/ReportDelegate.rb'\n",
    );
    let text = fix(root);
    assert!(text.contains("Renamed Report to ReportDelegate"), "{text}");
    let source =
        fs::read_to_string(root.join("server/delegates/ReportDelegate.rb")).expect("source");
    assert_eq!(
        source,
        "class ReportDelegate\n  def self.run\n  end\nend\n\nReportDelegate.run\n"
    );
}

#[test]
fn a_file_that_others_name_with_a_lowercase_name_is_a_manual_step() {
    let project = project();
    let root = project.path();
    write(
        root,
        "server/helpers/report.rb",
        "class ReportDelegate\nend\n",
    );
    write(
        root,
        "server/controllers/ReportController.js",
        "const script = 'server/helpers/report.rb'\n",
    );
    let text = fix(root);
    assert!(
        text.contains("Rename the file to 'server/helpers/ReportDelegate.rb'"),
        "{text}"
    );
    assert_eq!(names_in(root, "server/helpers"), ["report.rb"]);
}
