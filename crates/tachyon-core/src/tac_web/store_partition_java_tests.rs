#![allow(clippy::expect_used)]

use super::java_tests::{java_web_ready, publish, run_driver, write};
use super::store_partition_tests::StorePartitionProject;
use std::path::Path;

struct JavaStorePartitionProject;

impl JavaStorePartitionProject {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.java",
            "@Setup\nclass ChatSetup {\n    static Behaviors.Configuration configure(Behaviors.Execution execution) {\n        return new Behaviors.Configuration();\n    }\n}\n",
        ),
        (
            "client/entities/MessageEntity.java",
            "@Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})\n@Table(\"messages\")\nclass MessageEntity {\n    @Id\n    String id = \"\";\n    String room = \"\";\n    String text = \"\";\n}\n",
        ),
        (
            "client/stores/MessageStore.java",
            "@Store(MessageEntity.class, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})\ninterface MessageStore {\n    Object findByRoom(String room);\n}\n",
        ),
        (
            "client/services/ChatService.java",
            r#"import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Map;

@Service
class ChatService {
    Behaviors.Context behaviors;
    @Needs
    MessageStore messageStore;
    String heard = "";

    @OnStart
    void start() {
        behaviors = Behaviors.create("ChatSetup", new Behaviors.Options("chat", new Behaviors.Signal()));
    }

    String open(String room) {
        try {
            messages().load(room);
            return "";
        } catch (Behaviors.Failure failure) {
            return failed(failure);
        }
    }

    boolean isOpen(String room) {
        return messages().loaded(room);
    }

    String close(String room) {
        try {
            messages().unload(room);
            return "";
        } catch (Behaviors.Failure failure) {
            return failed(failure);
        }
    }

    String add(String id, String room, String text) {
        try {
            messages().save(Map.of("id", id, "room", room, "text", text));
            return "";
        } catch (Behaviors.Failure failure) {
            return failed(failure);
        }
    }

    String read(String room) {
        try {
            return ids(messageStore.findByRoom(room));
        } catch (Behaviors.Failure failure) {
            return failed(failure);
        }
    }

    @OnEvent
    void hearLoaded(StorePartitionLoaded loaded) {
        heard = loaded.table() + "/" + loaded.partition() + "/" + (long) loaded.rows();
    }

    private Behaviors.Repository messages() {
        return behaviors.repository("MessageEntity");
    }

    private static String failed(Behaviors.Failure failure) {
        return "error:" + failure.code;
    }

    private static String ids(Object rows) {
        List<String> found = new ArrayList<>();
        for (Object row : (List<?>) rows) {
            found.add(String.valueOf(((Map<?, ?>) row).get("id")));
        }
        Collections.sort(found);
        return String.join(",", found);
    }
}
"#,
        ),
    ];

    async fn built(root: &Path) {
        for (relative, text) in Self::FILES {
            write(root, relative, text);
        }
        publish(root, "chat", "ChatService").await;
    }
}

#[tokio::test]
async fn a_java_service_loads_one_partition_and_reads_it_from_memory() {
    if !java_web_ready() {
        eprintln!("skipping: TeaVM (TAC_TEAVM_HOME), javac or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    JavaStorePartitionProject::built(root.path()).await;
    StorePartitionProject::runtime_files(root.path());
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
