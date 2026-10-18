use super::yon_reach::YonReach;
use crate::ProjectDiscovery;
use std::fs;
use std::path::Path;
use tachyon_contracts::NativeTarget;

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("source");
}

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("project");
    write(root.path(), "client/pages/tac.html", "<main>Home</main>\n");
    for (relative, contents) in files {
        write(root.path(), relative, contents);
    }
    root
}

fn checked(root: &Path, target: NativeTarget) -> Result<Option<String>, crate::Failure> {
    let project = ProjectDiscovery::discover(root).expect("discovered");
    YonReach::check(&project, target)
}

struct Sources;

impl Sources {
    const CONTROLLER: (&str, &str) = (
        "server/controllers/HomeController.js",
        "@RequestRoute(\"/api/home\")\n@Public\nexport class HomeController {\n  @GetRoute\n  async get() {\n    return { ok: true }\n  }\n}\n",
    );
    const REMOTE: (&str, &str) = (
        "client/remotes/HomeClient.js",
        "@HTTPClient\nexport default class HomeClient {\n  /** @returns {Promise<BehaviorJsonValue>} */\n  @GetRequest(\"/api/home\")\n  async read() {}\n}\n",
    );
    const ORIGIN: (&str, &str) = (
        "client/manifest.json",
        "{\"tac\":{\"yon\":{\"origin\":\"https://api.example.com\"}}}\n",
    );
}

#[test]
fn an_app_without_yon_needs_no_origin_and_gets_no_note() {
    let root = project(&[]);
    assert_eq!(
        checked(root.path(), NativeTarget::Macos).expect("checked"),
        None
    );
    let hidden = project(&[("client/remotes/.gitkeep", "")]);
    assert_eq!(
        checked(hidden.path(), NativeTarget::Ios).expect("checked"),
        None
    );
}

#[test]
fn yon_routes_without_an_origin_make_an_offline_only_app_with_a_note() {
    let root = project(&[Sources::CONTROLLER]);
    let note = checked(root.path(), NativeTarget::Android)
        .expect("checked")
        .expect("note");
    assert!(
        note.starts_with("Note: the android app is offline-only."),
        "{note}"
    );
    assert!(note.contains("tac.yon.origin"), "{note}");
}

#[test]
fn pages_that_call_yon_without_an_origin_are_refused() {
    let nested = format!("{}/nested/HomeClient.js", crate::Project::REMOTE_ROOTS[1]);
    for remote in [Sources::REMOTE, (nested.as_str(), Sources::REMOTE.1)] {
        let root = project(&[Sources::CONTROLLER, remote]);
        let text = checked(root.path(), NativeTarget::Linux)
            .expect_err("refused")
            .to_string();
        assert!(text.contains("TY1601"), "{text}");
        assert!(
            text.contains("The linux app calls Yon through client/remotes"),
            "{text}"
        );
        assert!(text.contains("server.origins"), "{text}");
    }
}

#[test]
fn a_declared_origin_satisfies_every_native_target() {
    let root = project(&[Sources::CONTROLLER, Sources::REMOTE, Sources::ORIGIN]);
    for target in [
        NativeTarget::Macos,
        NativeTarget::Ios,
        NativeTarget::Android,
        NativeTarget::Windows,
        NativeTarget::Linux,
    ] {
        assert_eq!(checked(root.path(), target).expect("checked"), None);
    }
}
