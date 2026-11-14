use super::locks_tests::LocksTests;
use super::{WebApis, WebLanguage};

struct LocksRubyTests;

impl LocksRubyTests {
    const RUBY: &str = r#"#[Service]
class StampService
  attr_accessor :held

  def initialize
    @held = ""
  end

  def take
    Locks.new.request("outbox")
  end

  def give
    Locks.new.release("outbox")
  end

  # @param granted [LockGranted]
  #[OnEvent]
  def hear_granted(granted)
    @held = granted.name
  end
end
"#;
}

#[tokio::test]
async fn a_ruby_web_service_takes_and_releases_a_lock() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    if !WebApis::node_available() || unavailable.is_some() {
        eprintln!("skipping: node or the Ruby runtime is unavailable: {unavailable:?}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Ruby, "rb", LocksRubyTests::RUBY).await;
    LocksTests::assert_held(root.path(), true);
}
