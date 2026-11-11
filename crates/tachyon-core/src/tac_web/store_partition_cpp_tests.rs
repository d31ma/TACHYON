#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::cpp_stream_tests::CppStreams;
use super::store_partition_tests::StorePartitionProject;
use super::tests::{build_and_publish, run_driver, write_service};
use std::path::{Path, PathBuf};

struct CppStorePartitionProject;

impl CppStorePartitionProject {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.hpp",
            "class [[Setup]] ChatSetup {\npublic:\n    static BehaviorAdapters configure(const BehaviorExecution&) {\n        return BehaviorAdapters();\n    }\n};\n",
        ),
        (
            "client/entities/MessageEntity.hpp",
            "struct [[Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"}), Table(\"messages\")]] MessageEntity {\n    [[Id]] std::string id;\n    std::string room;\n    std::string text;\n};\n",
        ),
        (
            "client/stores/MessageStore.hpp",
            "class [[Store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})]] MessageStore : public tachyon::EntityRepository<MessageEntity> {\npublic:\n    std::vector<MessageEntity> findByRoom(const std::string& room);\n};\n",
        ),
        (
            "client/services/ChatService.hpp",
            r#"class [[Service]] ChatService {
public:
    [[Needs]] MessageStore& messages;
    std::string heard;

    std::string open(std::string room) {
        try {
            messages.load(room);
            return "";
        } catch (const tachyon::TacCodedError& problem) {
            return "error:" + problem.code();
        }
    }

    bool isOpen(std::string room) {
        try {
            return messages.loaded(room);
        } catch (const tachyon::TacCodedError&) {
            return false;
        }
    }

    std::string close(std::string room) {
        try {
            messages.unload(room);
            return "";
        } catch (const tachyon::TacCodedError& problem) {
            return "error:" + problem.code();
        }
    }

    std::string add(std::string id, std::string room, std::string text) {
        try {
            messages.save(MessageEntity{id, room, text});
            return "";
        } catch (const tachyon::TacCodedError& problem) {
            return "error:" + problem.code();
        }
    }

    std::string read(std::string room) {
        try {
            std::vector<std::string> ids;
            for (const MessageEntity& row : messages.findByRoom(room)) {
                ids.push_back(row.id);
            }
            std::sort(ids.begin(), ids.end());
            std::string joined;
            for (const std::string& id : ids) {
                joined += (joined.empty() ? "" : ",") + id;
            }
            return joined;
        } catch (const tachyon::TacCodedError& problem) {
            return "error:" + problem.code();
        }
    }

    [[OnEvent]] void hearLoaded(const StorePartitionLoaded& loaded) {
        heard = loaded.table + "/" + loaded.partition + "/" + std::to_string(static_cast<long long>(loaded.rows));
    }
};
"#,
        ),
    ];

    fn written(root: &Path) -> PathBuf {
        let mut service = PathBuf::new();
        for (relative, text) in Self::FILES {
            service = write_service(root, relative, text);
        }
        service
    }
}

#[tokio::test]
async fn a_cpp_service_loads_one_partition_and_reads_it_from_memory() {
    if !CppStreams::tools_available() {
        eprintln!("skipping: em++ (Emscripten) or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = CppStorePartitionProject::written(root.path());
    StorePartitionProject::runtime_files(root.path());
    build_and_publish(
        root.path(),
        "chat",
        "ChatService",
        WebLanguage::Cpp,
        service,
    )
    .await;
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
