#![allow(clippy::expect_used)]

use super::kotlin_tests::kotlin_web_ready;
use super::store_partition_tests::StorePartitionProject;
use super::tests::{run_driver, write_service};
use super::{BuildJobs, WebLanguage, build_all};
use std::path::Path;

struct KotlinStorePartitionProject;

impl KotlinStorePartitionProject {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.kt",
            "@Setup\nclass ChatSetup {\n    companion object {\n        \
             fun configure(execution: Behaviors.Execution): Behaviors.Configuration = Behaviors.Configuration()\n    }\n}\n",
        ),
        (
            "client/entities/MessageEntity.kt",
            "@Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})\n@Table(\"messages\")\nclass MessageEntity {\n    @Id\n    var id: String = \"\"\n    var room: String = \"\"\n    var text: String = \"\"\n}\n",
        ),
        (
            "client/stores/MessageStore.kt",
            "@Store(MessageEntity::class, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})\n\
             interface MessageStore {\n    \
             fun findByRoom(room: String): Any?\n}\n",
        ),
        (
            "client/services/ChatService.kt",
            r#"@Service
class ChatService {
    private lateinit var behaviors: Behaviors.Context
    @Needs
    private val messageStore: MessageStore
    var heard: String = ""

    @OnStart
    fun start() {
        behaviors = Behaviors.create("ChatSetup", Behaviors.Options("chat", Behaviors.Signal()))
    }

    fun open(room: String): String = attempt { messages().load(room) }

    fun isOpen(room: String): Boolean = messages().loaded(room)

    fun close(room: String): String = attempt { messages().unload(room) }

    fun add(id: String, room: String, text: String): String =
        attempt { messages().save(mapOf("id" to id, "room" to room, "text" to text)) }

    fun read(room: String): String {
        return try {
            ids(messageStore.findByRoom(room))
        } catch (error: Behaviors.Failure) {
            failed(error)
        }
    }

    @OnEvent
    fun hearLoaded(loaded: StorePartitionLoaded) {
        heard = "${loaded.table}/${loaded.partition}/${loaded.rows.toLong()}"
    }

    private fun messages(): Behaviors.Repository = behaviors.repository("MessageEntity")

    private fun attempt(work: () -> Any?): String {
        return try {
            work()
            ""
        } catch (error: Behaviors.Failure) {
            failed(error)
        }
    }

    private fun failed(error: Behaviors.Failure): String = "error:" + error.code

    private fun ids(rows: Any?): String {
        val found = (rows as? List<*>).orEmpty()
        return found.mapNotNull { row -> (row as? Map<*, *>)?.get("id") as? String }.sorted().joinToString(",")
    }
}
"#,
        ),
    ];

    async fn built(root: &Path) {
        let mut service = std::path::PathBuf::new();
        for (relative, text) in Self::FILES {
            service = write_service(root, relative, text);
        }
        let services = [(
            String::from("chat"),
            String::from("ChatService"),
            WebLanguage::Kotlin,
            service,
        )];
        let built = build_all(
            root,
            &services,
            &BuildJobs::new(1),
            crate::BuildProfile::Development,
        )
        .await
        .unwrap_or_else(|failure| panic!("{failure}"));
        for (name, bytes) in built.files {
            let path = root.join(name);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
            std::fs::write(path, bytes).expect("publish");
        }
    }
}

#[tokio::test]
async fn a_kotlin_service_loads_one_partition_and_reads_it_from_memory() {
    if !kotlin_web_ready() {
        eprintln!("skipping: kotlinc-js, the Kotlin wasm stdlib or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    KotlinStorePartitionProject::built(root.path()).await;
    StorePartitionProject::runtime_files(root.path());
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
