#![allow(clippy::expect_used)]

use super::dart_stream_tests::DartStreams;
use crate::tac_web::store_partition_tests::StorePartitionProject;

struct DartStorePartition;

impl DartStorePartition {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.dart",
            "@Setup\nclass ChatSetup {\n  static BehaviorAdapters configure(BehaviorExecution execution) {\n    return BehaviorAdapters();\n  }\n}\n",
        ),
        (
            "client/entities/MessageEntity.dart",
            "@Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})\n@Table(\"messages\")\nclass MessageEntity {\n  @Id\n  String id = '';\n  String room = '';\n  String text = '';\n}\n",
        ),
        (
            "client/stores/MessageStore.dart",
            "@Store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})\n\
             abstract interface class MessageStore {\n  Future<Object?> findByRoom(String room);\n}\n",
        ),
        (
            "client/services/ChatService.dart",
            r"@Service
class ChatService {
  @Needs
  final MessageStore messageStore;
  late BehaviorContext behaviors;
  String heard = '';
  String changed = '';

  @OnStart
  void start() {
    behaviors = Behaviors.create(ChatSetup, partition: 'chat', signal: BehaviorSignal());
  }

  Future<String> open(String room) => _finished(_messages().load(room));

  Future<bool> isOpen(String room) => _messages().loaded(room);

  Future<String> close(String room) => _finished(_messages().unload(room));

  Future<String> add(String id, String room, String text) =>
      _finished(_messages().save({'id': id, 'room': room, 'text': text}));

  Future<String> read(String room) async {
    try {
      return _ids(await messageStore.findByRoom(room));
    } on BehaviorError catch (error) {
      return _failed(error);
    }
  }

  @OnEvent
  void hearLoaded(StorePartitionLoaded loaded) {
    heard = '${loaded.table}/${loaded.partition}/${loaded.rows.toInt()}';
  }

  @OnEvent
  void hearChanged(StoreChanged change) {
    changed = '${change.table}/${change.partition}';
  }

  BehaviorRepository _messages() => behaviors.repository(MessageEntity);

  String _failed(BehaviorError error) => 'error:${error.code}';

  Future<String> _finished(Future<void> work) async {
    try {
      await work;
      return '';
    } on BehaviorError catch (error) {
      return _failed(error);
    }
  }

  String _ids(Object? rows) {
    final ids = [for (final row in rows as List) (row as Map)['id'] as String]..sort();
    return ids.join(',');
  }
}
",
        ),
    ];

    const SERVICES: [(&'static str, &'static str); 1] =
        [("ChatService", "client/services/ChatService.dart")];

    fn driver(base: &str) -> String {
        base.replace("= chat.", "= await chat.")
            .replace("\nchat.", "\nawait chat.")
    }

    async fn report(base: &str) -> Option<serde_json::Value> {
        if !DartStreams::tools_available() {
            eprintln!("skipping: dart or node is unavailable");
            return None;
        }
        let root = tempfile::tempdir().expect("project");
        DartStreams::publish(root.path(), &Self::FILES, &Self::SERVICES, &["chat"]).await;
        StorePartitionProject::runtime_files(root.path());
        Some(DartStreams::drive(root.path(), &Self::driver(base)))
    }
}

#[tokio::test]
async fn a_dart_service_loads_one_partition_and_reads_it_from_memory() {
    let report = DartStorePartition::report(StorePartitionProject::DRIVER).await;
    if let Some(report) = report {
        StorePartitionProject::assert_report(&report);
    }
}

#[tokio::test]
async fn a_dart_service_hears_a_store_change_from_another_tab() {
    let report = DartStorePartition::report(StorePartitionProject::TAB_DRIVER).await;
    if let Some(report) = report {
        StorePartitionProject::assert_tab_report(&report);
    }
}
