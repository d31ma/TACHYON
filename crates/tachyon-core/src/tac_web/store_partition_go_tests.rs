#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::store_partition_tests::StorePartitionProject;
use super::tests::{build_and_publish, node_available, run_driver, write_service};
use crate::required_toolchain::RequiredToolchain;
use std::path::{Path, PathBuf};

struct GoStorePartitionProject;

impl GoStorePartitionProject {
    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.go",
            "// @Setup\ntype ChatSetup struct{}\n\nfunc (ChatSetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {\n\treturn NewBehaviorConfiguration(), nil\n}\n",
        ),
        (
            "client/entities/MessageEntity.go",
            "// @Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})\n// @Table(\"messages\")\ntype MessageEntity struct {\n\t// @Id\n\tid   string\n\troom string\n\ttext string\n}\n",
        ),
        (
            "client/stores/MessageStore.go",
            "// @Store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})\ntype MessageStore interface {\n\tFindByRoom(room BehaviorValue) (BehaviorValue, error)\n}\n",
        ),
        (
            "client/services/ChatService.go",
            r#"import (
	"errors"
	"sort"
	"strconv"
	"strings"
)

// @Service
type ChatService struct {
	// @Needs
	messageStore *MessageStore
	behaviors    *BehaviorContext
	Heard        string
}

// @OnStart
func (chat *ChatService) Start() error {
	context, err := BehaviorsCreate[ChatSetup](NewBehaviorOptions("chat", NewBehaviorSignal()))
	chat.behaviors = context
	return err
}

func (chat *ChatService) Open(room string) string {
	return chat.answer(chat.messages().Load(room))
}

func (chat *ChatService) IsOpen(room string) bool {
	loaded, err := chat.messages().Loaded(room)
	return err == nil && loaded
}

func (chat *ChatService) Close(room string) string {
	return chat.answer(chat.messages().Unload(room))
}

func (chat *ChatService) Add(id string, room string, text string) string {
	_, err := chat.messages().Save(map[string]any{"id": id, "room": room, "text": text})
	return chat.answer(err)
}

func (chat *ChatService) Read(room string) string {
	rows, err := chat.messageStore.FindByRoom(room)
	if err != nil {
		return chat.answer(err)
	}
	return chat.ids(rows)
}

// @OnEvent
func (chat *ChatService) HearLoaded(loaded StorePartitionLoaded) {
	chat.Heard = loaded.Table + "/" + loaded.Partition + "/" + strconv.Itoa(int(loaded.Rows))
}

func (chat *ChatService) messages() *BehaviorRepository {
	repository, _ := BehaviorsRepository[MessageEntity](chat.behaviors)
	return repository
}

func (chat *ChatService) answer(err error) string {
	var failure *BehaviorError
	if errors.As(err, &failure) {
		return "error:" + failure.Code
	}
	if err != nil {
		return "error:" + err.Error()
	}
	return ""
}

func (chat *ChatService) ids(rows BehaviorValue) string {
	list, _ := rows.([]any)
	ids := []string{}
	for _, row := range list {
		object, _ := row.(map[string]any)
		id, _ := object["id"].(string)
		ids = append(ids, id)
	}
	sort.Strings(ids)
	return strings.Join(ids, ",")
}
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
async fn a_go_service_loads_one_partition_and_reads_it_from_memory() {
    if !RequiredToolchain::answers("go", "version") || !node_available() {
        eprintln!("skipping: go or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = GoStorePartitionProject::written(root.path());
    StorePartitionProject::runtime_files(root.path());
    build_and_publish(root.path(), "chat", "ChatService", WebLanguage::Go, service).await;
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
