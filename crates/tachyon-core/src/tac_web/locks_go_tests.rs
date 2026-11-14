use super::locks_tests::LocksTests;
use super::{WebApis, WebLanguage};
use std::process::Command;

struct LocksGoTests;

impl LocksGoTests {
    const GO: &str = r#"// @Service
type StampService struct {
	// @Needs
	locks Locks
	Held  string
}

func (stamp *StampService) Take() {
	_ = stamp.locks.Request("outbox")
}

func (stamp *StampService) Give() {
	_ = stamp.locks.Release("outbox")
}

// @OnEvent
func (stamp *StampService) HearGranted(granted LockGranted) {
	stamp.Held = granted.Name
}
"#;
}

#[tokio::test]
async fn a_go_web_service_takes_and_releases_a_lock() {
    let go = Command::new("go")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success());
    if !WebApis::node_available() || !go {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Go, "go", LocksGoTests::GO).await;
    LocksTests::assert_held(root.path(), true);
}
