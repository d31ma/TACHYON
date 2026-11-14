#![allow(clippy::expect_used)]

use super::MountPath;
use std::path::PathBuf;

fn chat() -> MountPath {
    MountPath::parse("/chat").expect("mount")
}

fn file(path: &str) -> (PathBuf, Vec<u8>) {
    (PathBuf::from(path), Vec::new())
}

fn placed(mount: &MountPath, path: &str) -> Vec<PathBuf> {
    mount
        .files(vec![file(path)])
        .into_iter()
        .map(|(path, _)| path)
        .collect()
}

#[test]
fn a_one_segment_mount_is_accepted() {
    assert_eq!(chat().prefix(), "/chat");
}

#[test]
fn a_four_segment_mount_is_accepted() {
    let mount = MountPath::parse("/a/b-c/d1/e").expect("mount");
    assert_eq!(mount.prefix(), "/a/b-c/d1/e");
}

#[test]
fn the_root_is_not_a_mount() {
    let error = MountPath::parse("/").expect_err("root");
    assert!(error.contains("tac.mount must be a path prefix"), "{error}");
}

#[test]
fn a_mount_needs_a_leading_slash() {
    assert!(MountPath::parse("chat").is_err());
}

#[test]
fn a_trailing_slash_is_refused() {
    assert!(MountPath::parse("/chat/").is_err());
}

#[test]
fn an_uppercase_segment_is_refused() {
    assert!(MountPath::parse("/Chat").is_err());
}

#[test]
fn a_parameter_segment_is_refused() {
    assert!(MountPath::parse("/chat/_id").is_err());
}

#[test]
fn five_segments_are_refused() {
    assert!(MountPath::parse("/a/b/c/d/e").is_err());
}

#[test]
fn the_shared_folder_is_reserved() {
    let error = MountPath::parse("/shared").expect_err("reserved");
    assert!(error.contains("cannot start with /shared"), "{error}");
}

#[test]
fn the_default_mount_is_the_root() {
    assert!(MountPath::default().is_root());
}

#[test]
fn the_index_route_becomes_the_prefix() {
    assert_eq!(chat().route("/"), "/chat");
}

#[test]
fn a_nested_route_keeps_its_segments_under_the_prefix() {
    assert_eq!(chat().route("/_id/edit"), "/chat/_id/edit");
}

#[test]
fn the_root_mount_keeps_the_index_route() {
    assert_eq!(MountPath::default().route("/"), "/");
}

#[test]
fn the_root_mount_keeps_a_route() {
    assert_eq!(MountPath::default().route("/items"), "/items");
}

#[test]
fn services_live_under_the_prefix() {
    assert_eq!(chat().services_folder(), "/chat/.tachyon/services/");
}

#[test]
fn root_services_keep_their_folder() {
    assert_eq!(
        MountPath::default().services_folder(),
        "/.tachyon/services/"
    );
}

#[test]
fn runtime_links_of_a_document_move_under_the_prefix() {
    let html =
        r#"<script src="/.tachyon/tac-client.js"></script><link href="/.tachyon/components.css">"#;
    assert_eq!(
        chat().document(html),
        r#"<script src="/chat/.tachyon/tac-client.js"></script><link href="/chat/.tachyon/components.css">"#
    );
}

#[test]
fn author_links_of_a_document_stay() {
    let html = r#"<link href="/shared/app.css"><a href="/mail">Mail</a>"#;
    assert_eq!(chat().document(html), html);
}

#[test]
fn the_root_mount_leaves_a_document_alone() {
    let html = r#"<script src="/.tachyon/tac-client.js"></script>"#;
    assert_eq!(MountPath::default().document(html), html);
}

#[test]
fn a_runtime_file_moves_under_the_prefix() {
    assert_eq!(
        placed(&chat(), ".tachyon/tac-client.js"),
        vec![PathBuf::from("chat/.tachyon/tac-client.js")]
    );
}

#[test]
fn a_shared_file_moves_under_the_prefix() {
    assert_eq!(
        placed(&chat(), "shared/logo.svg"),
        vec![PathBuf::from("chat/shared/logo.svg")]
    );
}

#[test]
fn a_route_document_already_under_the_prefix_stays() {
    assert_eq!(
        placed(&chat(), "chat/_id/index.html"),
        vec![PathBuf::from("chat/_id/index.html")]
    );
}

#[test]
fn the_route_manifest_stays_at_the_deploy_root() {
    assert_eq!(
        placed(&chat(), "route-manifest.json"),
        vec![PathBuf::from("route-manifest.json")]
    );
}

#[test]
fn api_documents_stay_at_the_deploy_root() {
    assert_eq!(
        placed(&chat(), "api-docs/index.html"),
        vec![PathBuf::from("api-docs/index.html")]
    );
}

#[test]
fn the_build_state_stays_at_the_deploy_root() {
    assert_eq!(
        placed(&chat(), ".tachyon/build-state.json"),
        vec![PathBuf::from(".tachyon/build-state.json")]
    );
}

#[test]
fn a_mounted_app_writes_no_service_worker() {
    assert!(placed(&chat(), "tachyon-sw.js").is_empty());
}

#[test]
fn a_mounted_app_writes_no_worker_registration() {
    assert!(placed(&chat(), ".tachyon/register-sw.js").is_empty());
}

#[test]
fn a_mounted_app_writes_no_web_app_manifest() {
    assert!(placed(&chat(), "manifest.webmanifest").is_empty());
}

#[test]
fn a_mounted_app_writes_no_site_map() {
    assert!(placed(&chat(), "sitemap.xml").is_empty());
}

#[test]
fn a_mounted_app_writes_no_robots_file() {
    assert!(placed(&chat(), "robots.txt").is_empty());
}

#[test]
fn a_mounted_app_writes_no_site_association() {
    assert!(placed(&chat(), ".well-known/assetlinks.json").is_empty());
}

#[test]
fn the_root_mount_keeps_the_service_worker() {
    assert_eq!(
        placed(&MountPath::default(), "tachyon-sw.js"),
        vec![PathBuf::from("tachyon-sw.js")]
    );
}

#[test]
fn the_root_mount_keeps_runtime_files_in_place() {
    assert_eq!(
        placed(&MountPath::default(), ".tachyon/tac-client.js"),
        vec![PathBuf::from(".tachyon/tac-client.js")]
    );
}
