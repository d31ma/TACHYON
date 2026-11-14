use super::locks_tests::LocksTests;
use super::{WebApis, WebLanguage};

struct LocksKotlinTests;

impl LocksKotlinTests {
    const KOTLIN: &str = r#"@Service
class StampService {
    private val locks = Locks()
    var held: String = ""

    fun take() {
        locks.request("outbox")
    }

    fun give() {
        locks.release("outbox")
    }

    @OnEvent
    fun hearGranted(granted: LockGranted) {
        held = granted.name
    }
}
"#;
}

#[tokio::test]
async fn a_kotlin_web_service_takes_and_releases_a_lock() {
    if !super::super::kotlin_tests::kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(
        root.path(),
        WebLanguage::Kotlin,
        "kt",
        LocksKotlinTests::KOTLIN,
    )
    .await;
    LocksTests::assert_held(root.path(), true);
}
