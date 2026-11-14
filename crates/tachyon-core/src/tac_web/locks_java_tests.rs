use super::locks_tests::LocksTests;
use super::publish_java;

struct LocksJavaTests;

impl LocksJavaTests {
    const JAVA: &str = r#"@Service
class StampService {
    private final Locks locks = new Locks();
    String held = "";

    void take() {
        locks.request("outbox");
    }

    void give() {
        locks.release("outbox");
    }

    @OnEvent
    void hearGranted(LockGranted granted) {
        held = granted.name();
    }
}
"#;
}

#[tokio::test]
async fn a_java_web_service_takes_and_releases_a_lock() {
    let root = tempfile::tempdir().expect("project");
    if !publish_java(root.path(), LocksJavaTests::JAVA).await {
        return;
    }
    LocksTests::assert_held(root.path(), true);
}
