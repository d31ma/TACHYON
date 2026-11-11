#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::store_partition_tests::StorePartitionProject;
use super::tests::{build_and_publish, node_available, run_driver, write_service};
use std::path::{Path, PathBuf};

struct RubyStorePartition;

impl RubyStorePartition {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.rb",
            "#[Setup]\nclass ChatSetup\n  def self.configure(_execution)\n    {}\n  end\nend\n",
        ),
        (
            "client/entities/MessageEntity.rb",
            "#[Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})]\n#[Table(\"messages\")]\nclass MessageEntity\n  # @return [String]\n  #[Id]\n  attr_reader :id\n\n  # @return [String]\n  attr_reader :room\n\n  # @return [String]\n  attr_reader :text\nend\n",
        ),
        (
            "client/stores/MessageStore.rb",
            "#[Store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})]\nclass MessageStore\n  def find_by_room(room)\n  end\nend\n",
        ),
        (
            "client/services/ChatService.rb",
            r##"#[Service]
class ChatService
  #[Needs]
  # @return [MessageStore]
  attr_reader :message_store

  # @return [String]
  attr_reader :heard

  def initialize
    @heard = ""
  end

  #[OnStart]
  def start
    @behaviors = Behaviors.create(ChatSetup, partition: "chat", signal: BehaviorSignal.new)
  end

  # @param room [String]
  # @return [String]
  def open(room)
    @behaviors.repository(MessageEntity).load(room)
    ""
  rescue BehaviorError => error
    "error:#{error.behavior_code}"
  end

  # @param room [String]
  # @return [Boolean]
  def is_open(room)
    @behaviors.repository(MessageEntity).loaded(room)
  rescue BehaviorError
    false
  end

  # @param room [String]
  # @return [String]
  def close(room)
    @behaviors.repository(MessageEntity).unload(room)
    ""
  rescue BehaviorError => error
    "error:#{error.behavior_code}"
  end

  # @param id [String]
  # @param room [String]
  # @param text [String]
  # @return [String]
  def add(id, room, text)
    @behaviors.repository(MessageEntity).save({ "id" => id, "room" => room, "text" => text })
    ""
  rescue BehaviorError => error
    "error:#{error.behavior_code}"
  end

  # @param room [String]
  # @return [String]
  def read(room)
    ids = message_store.find_by_room(room).map { |row| row["id"] }
    ids.sort.join(",")
  rescue BehaviorError => error
    "error:#{error.behavior_code}"
  end

  # @param loaded [StorePartitionLoaded]
  #[OnEvent]
  def hear_loaded(loaded)
    @heard = "#{loaded.table}/#{loaded.partition}/#{loaded.rows.to_i}"
  end
end
"##,
        ),
    ];

    fn skip_reason() -> Option<String> {
        if !node_available() {
            return Some(String::from("node is unavailable"));
        }
        let reason = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
        assert!(
            reason.is_none() || std::env::var_os("CI").is_none(),
            "CI installs Emscripten and Ruby to build the pinned mruby runtime, so the Ruby Tac tests must run there: {reason:?}"
        );
        reason
    }

    fn written(root: &Path) -> PathBuf {
        let mut service = PathBuf::new();
        for (relative, text) in Self::FILES {
            service = write_service(root, relative, text);
        }
        service
    }
}

#[tokio::test]
async fn a_ruby_service_loads_one_partition_and_reads_it_from_memory() {
    if let Some(reason) = RubyStorePartition::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = RubyStorePartition::written(root.path());
    StorePartitionProject::runtime_files(root.path());
    build_and_publish(
        root.path(),
        "chat",
        "ChatService",
        WebLanguage::Ruby,
        service,
    )
    .await;
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
