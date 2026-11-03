use super::DeclaredHosts;
use cap_std::fs::Dir;

fn project(manifest: &serde_json::Value) -> (tempfile::TempDir, Dir) {
    let root = tempfile::tempdir().expect("project");
    std::fs::create_dir_all(root.path().join("client")).expect("client");
    std::fs::write(
        root.path().join("client/manifest.json"),
        manifest.to_string(),
    )
    .expect("manifest");
    std::fs::write(
        root.path().join("manifest.webmanifest"),
        manifest.to_string(),
    )
    .expect("published");
    let directory = Dir::open_ambient_dir(root.path(), cap_std::ambient_authority()).expect("dir");
    (root, directory)
}

#[test]
fn the_page_may_connect_to_the_declared_yon_origin_and_its_sockets() {
    let (root, directory) = project(&serde_json::json!({ "tac": {
        "outgoing": { "hosts": ["maps.example.com"] },
        "yon": { "origin": "https://api.example.com" },
    } }));
    let expected = "https://maps.example.com wss://maps.example.com http://maps.example.com ws://maps.example.com https://api.example.com wss://api.example.com";
    assert_eq!(DeclaredHosts::tac(&directory).connect_sources(), expected);
    assert_eq!(
        DeclaredHosts::published(root.path()).connect_sources(),
        expected
    );
}

#[test]
fn a_local_yon_origin_keeps_its_port_and_uses_plain_sockets() {
    let (_root, directory) = project(&serde_json::json!({ "tac": {
        "yon": { "origin": "http://127.0.0.1:8080" },
    } }));
    assert_eq!(
        DeclaredHosts::tac(&directory).connect_sources(),
        "http://127.0.0.1:8080 ws://127.0.0.1:8080"
    );
}

#[test]
fn an_unreadable_yon_origin_adds_nothing() {
    for origin in [
        serde_json::json!("not an origin"),
        serde_json::json!(8080),
        serde_json::json!("tachyon-app://bundle"),
    ] {
        let (_root, directory) =
            project(&serde_json::json!({ "tac": { "yon": { "origin": origin } } }));
        assert_eq!(
            DeclaredHosts::tac(&directory).connect_sources(),
            "",
            "{origin}"
        );
    }
}
