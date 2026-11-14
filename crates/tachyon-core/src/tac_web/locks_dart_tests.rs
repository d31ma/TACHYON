use super::WebApis;
use super::locks_tests::LocksTests;

struct LocksDartTests;

impl LocksDartTests {
    const DART: &str = r"@Service()
class StampService {
  @Needs()
  final Locks locks;
  String held = '';

  void take() => locks.request('outbox');

  void give() => locks.release('outbox');

  @OnEvent
  void hearGranted(LockGranted granted) {
    held = granted.name;
  }
}
";
}

#[tokio::test]
async fn a_dart_service_takes_and_releases_a_lock() {
    if !WebApis::node_available() || !WebApis::dart_available() {
        eprintln!("skipping: node or dart is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish_dart(root.path(), LocksDartTests::DART).await;
    LocksTests::assert_held(root.path(), true);
}
