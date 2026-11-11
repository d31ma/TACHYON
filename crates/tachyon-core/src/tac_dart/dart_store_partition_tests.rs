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

    fn driver() -> String {
        StorePartitionProject::DRIVER
            .replace("= chat.", "= await chat.")
            .replace("\nchat.", "\nawait chat.")
    }
}

#[tokio::test]
async fn a_dart_service_loads_one_partition_and_reads_it_from_memory() {
    if !DartStreams::tools_available() {
        eprintln!("skipping: dart or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    DartStreams::publish(
        root.path(),
        &DartStorePartition::FILES,
        &DartStorePartition::SERVICES,
        &["chat"],
    )
    .await;
    StorePartitionProject::runtime_files(root.path());
    let report = DartStreams::drive(root.path(), &DartStorePartition::driver());
    StorePartitionProject::assert_report(&report);
}
