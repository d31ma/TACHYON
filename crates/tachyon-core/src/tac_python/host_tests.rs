use std::path::Path;

struct PythonHostFixture;

impl PythonHostFixture {
    async fn run(root: &Path) {
        let script = root.join("ownership.cjs");
        let host = root.join("python_host.js");
        std::fs::write(&script, include_str!("host_ownership_test.cjs"))
            .unwrap_or_else(|error| panic!("ownership fixture: {error}"));
        std::fs::write(&host, include_str!("python_host.js"))
            .unwrap_or_else(|error| panic!("production host: {error}"));
        let mut command = tokio::process::Command::new("node");
        command.arg(&script).arg(&host);
        let output = crate::external_command::run(
            &mut command,
            std::time::Duration::from_secs(10),
            64 * 1024,
        )
        .await
        .unwrap_or_else(|error| panic!("supervised ownership probe: {error}"));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[tokio::test]
async fn owned_python_methods_survive_parent_release_and_dispose_exactly_once() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("fixture: {error}"));
    PythonHostFixture::run(root.path()).await;
}
