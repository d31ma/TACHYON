use super::locks_tests::LocksTests;
use super::{WebApis, WebLanguage};

struct LocksCppTests;

impl LocksCppTests {
    const SOURCE: &str = r#"#pragma once
#include <string>

class [[Service]] StampService {
public:
    [[Needs]] Locks& locks;
    std::string held;

    void take() { locks.request("outbox"); }

    void give() { locks.release("outbox"); }

    [[OnEvent]] void hearGranted(const LockGranted& granted) { held = granted.name; }
};
"#;
}

#[tokio::test]
async fn a_cpp_web_service_takes_and_releases_a_lock() {
    if !WebApis::node_available() || !WebApis::cpp_available() {
        eprintln!("skipping: node or em++ is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Cpp, "hpp", LocksCppTests::SOURCE).await;
    LocksTests::assert_held(root.path(), true);
}
