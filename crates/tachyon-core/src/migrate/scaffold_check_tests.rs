use super::{MigrationAnalysis, MigrationFinding, MigrationStatus};
use std::fs;
use std::path::Path;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, contents).expect("file");
}

fn findings(root: &Path, feature: &str) -> Vec<MigrationFinding> {
    MigrationAnalysis::check(root)
        .expect("check")
        .findings
        .into_iter()
        .filter(|finding| finding.feature == feature)
        .collect()
}

#[test]
fn check_reports_plain_page_markup_that_bundle_refuses() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    write(
        root,
        "client/pages/tac.html",
        "<div class=\"shell\"><hero /></div>\n",
    );
    write(
        root,
        "client/pages/about/tac.html",
        "<!-- about -->\n<hero />\n",
    );
    write(
        root,
        "client/components/hero/tac.html",
        "<section>Hero</section>\n",
    );

    let found = findings(root, "view.page_markup");
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].source, "client/pages/tac.html");
    assert_eq!(found[0].status, MigrationStatus::Changed);
    assert!(found[0].detail.contains("TY1309"), "{found:?}");
    assert!(found[0].detail.contains("a plain element"), "{found:?}");
    let action = found[0].action.as_deref().unwrap_or_default();
    assert!(
        action.contains("client/components/page/home/tac.html") && action.contains("<page-home />"),
        "{action}"
    );
}

#[test]
fn check_reports_env_names_that_start_refuses_and_what_to_rename_them_to() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    write(root, ".env", "DATABASE_URL=very-secret\nYON_API_KEY=fine\n");
    write(
        root,
        ".env.example",
        "YON_PORT=8000\nYON_SKIP_BUNDLE=false\n",
    );

    let found = findings(root, "environment.env_name");
    let sources: Vec<&str> = found
        .iter()
        .map(|finding| finding.source.as_str())
        .collect();
    assert_eq!(sources, [".env.example:1", ".env.example:2", ".env:1"]);
    assert!(
        found[2]
            .detail
            .starts_with("ty start refuses the name 'DATABASE_URL' (TY2010)")
    );
    assert_eq!(
        found[2].action.as_deref(),
        Some(
            "Rename it to YON_DATABASE_URL in .env and keep its value; the server reads it as yon.database.url."
        )
    );
    assert!(found[0].detail.contains("copied to .env"), "{found:?}");
    assert!(
        found[0]
            .action
            .as_deref()
            .is_some_and(|action| action.contains("set server.port in server/application.json")),
        "{found:?}"
    );
    assert!(
        found[1]
            .action
            .as_deref()
            .is_some_and(|action| action.contains("--no-bundle")),
        "{found:?}"
    );
    let rendered = format!("{found:?}");
    assert!(
        !rendered.contains("very-secret"),
        "values never appear: {rendered}"
    );
}
