use super::renames::FileMove;
use super::{MigrationFix, MigrationFixPlan};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

struct OldPage;

impl OldPage {
    const SHELL: &'static str =
        "<div class=\"shell\">\n  <strong>Tachyon</strong>\n  <slot />\n  <hero />\n</div>\n";
    const STYLE: &'static str = "body { margin: 0; background: #0f172a; color: #e2e8f0; }\n.shell { max-width: 72rem; margin: 0 auto; }\n";
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("folders");
    fs::write(path, contents).expect("file");
}

fn read(root: &Path, relative: &str) -> String {
    fs::read_to_string(root.join(relative)).unwrap_or_else(|_| panic!("{relative}"))
}

fn plan(root: &Path) -> MigrationFixPlan {
    MigrationFix::plan(root).expect("plan")
}

fn old_page() -> TempDir {
    let project = tempfile::tempdir().expect("project");
    write(project.path(), "client/pages/tac.html", OldPage::SHELL);
    write(project.path(), "client/pages/tac.css", OldPage::STYLE);
    write(
        project.path(),
        "client/pages/tac.js",
        "document.title = \"Drill\"\n",
    );
    write(
        project.path(),
        "client/components/hero/tac.html",
        "<section>Hero</section>\n",
    );
    project
}

#[test]
fn page_markup_moves_into_a_page_component_that_the_page_places() {
    let project = old_page();
    let root = project.path();
    plan(root).write(root).expect("write");

    assert_eq!(read(root, "client/pages/tac.html"), "<page-home />\n");
    assert_eq!(
        read(root, "client/components/page/home/tac.html"),
        OldPage::SHELL
    );
    assert!(!root.join("client/pages/tac.css").exists());
    assert!(!root.join("client/pages/tac.js").exists());
}

#[test]
fn a_page_stylesheet_splits_into_document_styles_and_component_styles() {
    let project = old_page();
    let root = project.path();
    plan(root).write(root).expect("write");

    assert_eq!(
        read(root, "client/shared/styles/home-page.css"),
        "body { margin: 0; background: #0f172a; color: #e2e8f0; }\n"
    );
    assert_eq!(
        read(root, "client/components/page/home/tac.css"),
        ".shell { max-width: 72rem; margin: 0 auto; }\n"
    );
}

#[test]
fn the_page_script_moves_to_shared_scripts_and_a_new_manifest_lists_both_files() {
    let project = old_page();
    let root = project.path();
    plan(root).write(root).expect("write");

    assert_eq!(
        read(root, "client/shared/scripts/home-page.js"),
        "document.title = \"Drill\"\n"
    );
    let manifest: serde_json::Value =
        serde_json::from_str(&read(root, "client/manifest.json")).expect("manifest");
    assert_eq!(
        manifest,
        serde_json::json!({"tac": {
            "scripts": ["/shared/scripts/home-page.js"],
            "styles": ["/shared/styles/home-page.css"]
        }})
    );
}

#[test]
fn a_second_run_after_the_page_moved_changes_nothing() {
    let project = old_page();
    let root = project.path();
    plan(root).write(root).expect("write");

    let again = plan(root);
    assert_eq!(
        [
            again.change_count(),
            again.move_count(),
            again.manual_step_count()
        ],
        [0, 0, 0],
        "{}",
        again.to_text(false)
    );
}

#[test]
fn an_existing_manifest_is_left_alone_with_a_manual_step() {
    let project = old_page();
    let root = project.path();
    write(root, "client/manifest.json", "{\"name\": \"Drill\"}\n");
    let planned = plan(root);
    let text = planned.to_text(false);
    assert!(text.contains("manual   client/manifest.json"), "{text}");
    assert!(
        text.contains("'/shared/scripts/home-page.js' to tac.scripts"),
        "{text}"
    );
    planned.write(root).expect("write");
    assert_eq!(
        read(root, "client/manifest.json"),
        "{\"name\": \"Drill\"}\n"
    );
}

#[test]
fn a_page_script_stays_when_the_app_has_more_than_one_page() {
    let project = old_page();
    let root = project.path();
    write(root, "client/pages/about/tac.html", "<hero />\n");
    let planned = plan(root);
    let text = planned.to_text(false);
    assert!(text.contains("manual   client/pages/tac.js"), "{text}");
    assert!(text.contains("every page of this app"), "{text}");
    planned.write(root).expect("write");
    assert!(root.join("client/pages/tac.js").exists());
    assert_eq!(read(root, "client/pages/about/tac.html"), "<hero />\n");
}

#[test]
fn a_page_with_a_state_script_or_a_taken_component_gets_a_manual_step() {
    let state = tempfile::tempdir().expect("project");
    write(
        state.path(),
        "client/pages/tac.html",
        "<script>let count = 0</script><p>{count}</p>\n",
    );
    let text = plan(state.path()).to_text(false);
    assert!(text.contains("holds a state script"), "{text}");

    let taken = old_page();
    write(
        taken.path(),
        "client/components/page/home/tac.html",
        "<p>Taken</p>\n",
    );
    let text = plan(taken.path()).to_text(false);
    assert!(
        text.contains("'client/components/page/home' already exists"),
        "{text}"
    );
    assert!(!text.contains("moved    client/pages/tac.css"), "{text}");
}

#[test]
fn nested_page_folders_name_their_component_after_the_route() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    write(
        root,
        "client/pages/blog/_slug/tac.html",
        "<article>Post</article>\n",
    );
    plan(root).write(root).expect("write");

    assert_eq!(
        read(root, "client/pages/blog/_slug/tac.html"),
        "<page-blog-slug />\n"
    );
    assert_eq!(
        read(root, "client/components/page/blog-slug/tac.html"),
        "<article>Post</article>\n"
    );
}

#[test]
fn a_page_that_only_composes_components_is_not_moved() {
    let project = tempfile::tempdir().expect("project");
    let root = project.path();
    write(root, "client/pages/tac.html", "<!-- home -->\n<hero />\n");
    write(
        root,
        "client/components/hero/tac.html",
        "<section>Hero</section>\n",
    );
    let planned = plan(root);
    assert_eq!(planned.change_count(), 0, "{}", planned.to_text(false));
}

#[test]
fn the_write_guard_moves_only_page_companions_to_their_planned_places() {
    let allowed = [
        (
            "client/pages/tac.css",
            "client/components/page/home/tac.css",
        ),
        ("client/pages/tac.css", "client/shared/styles/home-page.css"),
        ("client/pages/tac.js", "client/shared/scripts/home-page.js"),
        (
            "client/pages/blog/tac.js",
            "client/shared/scripts/blog-page.js",
        ),
    ];
    let refused = [
        ("client/pages/tac.css", "client/shared/styles/app.css"),
        ("client/pages/tac.js", "client/shared/scripts/blog-page.js"),
        (
            "client/pages/tac.html",
            "client/components/page/home/tac.html",
        ),
        (
            "client/pages/Blog/tac.js",
            "client/shared/scripts/Blog-page.js",
        ),
        (
            "client/components/hero/tac.css",
            "client/shared/styles/hero-page.css",
        ),
    ];
    for (pairs, expected) in [(&allowed[..], true), (&refused[..], false)] {
        for (from, to) in pairs {
            let file_move = FileMove {
                from: String::from(*from),
                to: String::from(*to),
            };
            assert_eq!(
                MigrationFix::is_allowed_move(&file_move),
                expected,
                "{from} -> {to}"
            );
        }
    }
    for relative in [
        "client/pages/tac.css",
        "client/shared/styles/home-page.css",
        "client/components/page/home/tac.css",
        ".env.example",
    ] {
        assert!(MigrationFix::is_writable_target(relative), "{relative}");
    }
    for relative in [
        ".env",
        "client/shared/styles/app.css",
        "../client/pages/tac.css",
    ] {
        assert!(!MigrationFix::is_writable_target(relative), "{relative}");
    }
}
