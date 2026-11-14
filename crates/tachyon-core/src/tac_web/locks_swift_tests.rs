use super::locks_tests::LocksTests;
use super::publish_swift;

struct LocksSwiftTests;

impl LocksSwiftTests {
    const SWIFT: &str = r#"@Service
final class StampService {
    private let locks = Locks()
    var held: String = ""

    func take() throws {
        try locks.request("outbox")
    }

    func give() throws {
        try locks.release("outbox")
    }

    @OnEvent
    func hearGranted(_ granted: LockGranted) {
        held = granted.name
    }
}
"#;
}

#[tokio::test]
async fn a_swift_web_service_takes_and_releases_a_lock() {
    let root = tempfile::tempdir().expect("project");
    if !publish_swift(root.path(), LocksSwiftTests::SWIFT).await {
        return;
    }
    LocksTests::assert_held(root.path(), true);
}
