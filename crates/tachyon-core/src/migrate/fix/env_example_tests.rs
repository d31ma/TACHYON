use super::{MigrationFix, MigrationFixPlan};
use std::fs;
use std::path::Path;

struct Example;

impl Example {
    const TEXT: &'static str = "YON_PORT=8000\nYON_HOST=127.0.0.1\n# kept comment\nDATABASE_URL=\"postgres://example\"\nYON_DATABASE_PASSWORD=replace-me\n";
}

fn plan(root: &Path) -> MigrationFixPlan {
    MigrationFix::plan(root).expect("plan")
}

#[test]
fn env_example_names_are_renamed_or_commented_out_with_values_unchanged() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    fs::write(root.join(".env.example"), Example::TEXT).expect("example");
    plan(root).write(root).expect("write");

    let written = fs::read_to_string(root.join(".env.example")).expect("example");
    assert_eq!(
        written,
        "# ty start refuses YON_PORT in .env; set server.port in server/application.json, or pass --port to ty start.\n# YON_PORT=8000\n# ty start refuses YON_HOST in .env; set server.host in server/application.json.\n# YON_HOST=127.0.0.1\n# kept comment\nYON_DATABASE_URL=\"postgres://example\"\nYON_DATABASE_PASSWORD=replace-me\n"
    );
    assert_eq!(
        plan(root).change_count(),
        0,
        "a second run finds nothing to rename"
    );
}

#[test]
fn env_is_never_written_and_each_refused_name_is_a_manual_step() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    let secrets = "YON_PORT=9000\ndatabase_url=secret-value\nYON_API_KEY=kept\n";
    fs::write(root.join(".env"), secrets).expect("secrets");
    let planned = plan(root);
    let text = planned.to_text(false);
    assert!(text.contains("manual   .env:1"), "{text}");
    assert!(
        text.contains("Remove it from .env, then set server.port"),
        "{text}"
    );
    assert!(text.contains("manual   .env:2"), "{text}");
    assert!(
        text.contains("Rename it to YON_DATABASE_URL in .env"),
        "{text}"
    );
    assert!(!text.contains(".env:3"), "{text}");
    assert!(
        !text.contains("secret-value"),
        "values never appear: {text}"
    );
    planned.write(root).expect("write");
    assert_eq!(
        fs::read_to_string(root.join(".env")).expect("secrets"),
        secrets
    );
}

#[test]
fn a_name_that_becomes_a_reserved_name_is_commented_out_not_renamed() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    fs::write(root.join(".env.example"), "PORT=3000\nTAC_API_KEY=abc\n").expect("example");
    plan(root).write(root).expect("write");
    assert_eq!(
        fs::read_to_string(root.join(".env.example")).expect("example"),
        "# ty start refuses PORT in .env; set server.port in server/application.json, or pass --port to ty start.\n# PORT=3000\nYON_API_KEY=abc\n"
    );
}
