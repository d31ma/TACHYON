#![allow(clippy::expect_used)]

use super::store_partition_tests::StorePartitionProject;
use super::swift_tests::{publish, run_driver, swift_web_ready, write};

struct SwiftStorePartitionProject;

impl SwiftStorePartitionProject {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.swift",
            "@Setup\nfinal class ChatSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        BehaviorAdapters()\n    }\n}\n",
        ),
        (
            "client/entities/MessageEntity.swift",
            "@Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})\n@Table(\"messages\")\nfinal class MessageEntity {\n    @Id\n    var id: String = \"\"\n    var room: String = \"\"\n    var text: String = \"\"\n}\n",
        ),
        (
            "client/stores/MessageStore.swift",
            "@Store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})\nprotocol MessageStore {\n    func findByRoom(_ room: BehaviorJSON) async throws -> BehaviorJSON\n}\n",
        ),
        (
            "client/services/ChatService.swift",
            r#"@Service
final class ChatService {
    @Needs
    let messageStore: MessageStore
    private var behaviors: BehaviorContext!
    var heard: String = ""

    @OnStart
    func start() {
        behaviors = try? Behaviors.create(ChatSetup.self, partition: "chat", signal: BehaviorSignal())
    }

    func open(_ room: String) async -> String {
        do {
            try await messages().load(.string(room))
            return ""
        } catch {
            return Self.failed(error)
        }
    }

    func isOpen(_ room: String) async -> Bool {
        (try? await messages().loaded(.string(room))) ?? false
    }

    func close(_ room: String) async -> String {
        do {
            try await messages().unload(.string(room))
            return ""
        } catch {
            return Self.failed(error)
        }
    }

    func add(_ id: String, _ room: String, _ text: String) async -> String {
        do {
            _ = try await messages().save(.object(["id": .string(id), "room": .string(room), "text": .string(text)]))
            return ""
        } catch {
            return Self.failed(error)
        }
    }

    func read(_ room: String) async -> String {
        do {
            return try Self.ids(await messageStore.findByRoom(.string(room)))
        } catch {
            return Self.failed(error)
        }
    }

    @OnEvent
    func hearLoaded(_ loaded: StorePartitionLoaded) {
        heard = loaded.table + "/" + loaded.partition + "/" + String(Int(loaded.rows))
    }

    private func messages() throws -> BehaviorRepository {
        try behaviors.repository(MessageEntity.self)
    }

    private static func failed(_ error: Error) -> String {
        "error:" + ((error as? BehaviorError)?.code ?? String(describing: error))
    }

    private static func ids(_ rows: BehaviorJSON) throws -> String {
        try rows.items.map { try $0.get("id").text }.sorted().joined(separator: ",")
    }
}
"#,
        ),
    ];

    fn driver() -> String {
        StorePartitionProject::DRIVER.replace("chat.", "await chat.")
    }
}

#[tokio::test]
async fn a_swift_service_loads_one_partition_and_reads_it_from_memory() {
    if !swift_web_ready() {
        eprintln!("skipping: the Swift SDK for WebAssembly, swiftc or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    for (relative, text) in SwiftStorePartitionProject::FILES {
        write(root.path(), relative, text);
    }
    StorePartitionProject::runtime_files(root.path());
    publish(root.path(), "chat", "ChatService").await;
    let report = run_driver(root.path(), &SwiftStorePartitionProject::driver());
    StorePartitionProject::assert_report(&report);
}
