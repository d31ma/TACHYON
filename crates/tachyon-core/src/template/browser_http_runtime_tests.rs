use crate::node_driver::NodeDriver;
use std::path::Path;

#[test]
fn the_browser_http_host_passes_its_runtime_check() {
    if !NodeDriver::available() {
        eprintln!("skipping: node is unavailable");
        return;
    }
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../scripts/browser-http-runtime-test.mjs");
    let mut command = NodeDriver::command();
    command.arg(script);
    let output = NodeDriver::output(&mut command).expect("node browser-http-runtime-test.mjs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success(),
        "{stdout}{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout.contains("JSON error bodies pass"), "{stdout}");
}
