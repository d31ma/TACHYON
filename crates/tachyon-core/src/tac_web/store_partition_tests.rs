#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::tests::{
    build_and_publish, node_available, run_driver, web_assembly_target_installed, write_service,
};
use std::path::{Path, PathBuf};

pub(crate) struct StorePartitionProject;

impl StorePartitionProject {
    pub(crate) const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
globalThis.fetch = async (url) => new Response(await readFile(url))
console.info = () => {}
;(0, eval)(`${await readFile('./memory-indexeddb.js', 'utf8')}\nglobalThis.MemoryIndexedDb = MemoryIndexedDb`)
MemoryIndexedDb.install()
MemoryIndexedDb.seed('tachyon-store', 'messages', 'id', 'room', [{ id: 'm1', room: 'r1', text: 'one' }, { id: 'm2', room: 'r1', text: 'two' }, { id: 'm3', room: 'r2', text: 'three' }])
globalThis.document = { visibilityState: 'visible', addEventListener: () => {}, querySelector: () => null }
;(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-storage.js', 'utf8'))
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
await globalThis[Symbol.for('tachyon.tac.storage')].mirror()
const { default: ChatService, __tachyon_event_plan__: plan } = await import('./ChatService.js')
const chat = new ChatService({})
bus.attach(chat, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(chat)
const settle = async () => { for (let round = 0; round < 40; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const before = chat.read('r1')
const openBefore = chat.isOpen('r1')
chat.open('r1')
chat.add('m4', 'r1', 'late')
chat.add('m9', 'r3', 'cold')
await settle()
const heard = chat.heard
const after = chat.read('r1')
const openAfter = chat.isOpen('r1')
chat.close('r1')
const closed = chat.read('r1')
await settle()
const cold = MemoryIndexedDb.rows('tachyon-store', 'messages').filter((row) => row.room === 'r3').map((row) => row.id)
console.log(JSON.stringify({ before, openBefore, heard, after, openAfter, closed, cold }))
process.exit(0)
";

    const RUST_FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.rs",
            "#[Setup]\nstruct ChatSetup {}\nimpl ChatSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n",
        ),
        (
            "client/entities/MessageEntity.rs",
            "#[Entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})]\n#[Table(\"messages\")]\nstruct MessageEntity {\n    #[Id]\n    id: String,\n    room: String,\n    text: String,\n}\n",
        ),
        (
            "client/stores/MessageStore.rs",
            "#[Store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})]\ntrait MessageStore {\n    fn find_by_room(&self, room: BehaviorValue) -> BehaviorResult<BehaviorValue>;\n}\n",
        ),
        (
            "client/services/ChatService.rs",
            r#"#[Service]
pub struct ChatService {
    message_store: std::sync::Arc<MessageStore>,
    behaviors: BehaviorContext,
    pub heard: String,
}

impl ChatService {
    fn new() -> Self {
        let options = BehaviorOptions::new("chat", BehaviorSignal::new());
        Self {
            message_store: std::sync::Arc::default(),
            behaviors: Behaviors::create::<ChatSetup>(options).expect("behavior context"),
            heard: String::new(),
        }
    }

    fn failed(error: &BehaviorError) -> String {
        format!("error:{}", error.code)
    }

    fn ids(rows: &BehaviorValue) -> String {
        let BehaviorValue::Array(rows) = rows else {
            return String::new();
        };
        let mut ids: Vec<String> = rows
            .iter()
            .filter_map(|row| match row.get("id") {
                Some(BehaviorValue::String(id)) => Some(id.clone()),
                _ => None,
            })
            .collect();
        ids.sort();
        ids.join(",")
    }

    pub fn open(&self, room: String) -> String {
        let result = self.behaviors.repository::<MessageEntity>().and_then(|messages| messages.load(room.as_str()));
        result.err().map(|error| Self::failed(&error)).unwrap_or_default()
    }

    pub fn is_open(&self, room: String) -> bool {
        self.behaviors.repository::<MessageEntity>().and_then(|messages| messages.loaded(room.as_str())).unwrap_or(false)
    }

    pub fn close(&self, room: String) -> String {
        let result = self.behaviors.repository::<MessageEntity>().and_then(|messages| messages.unload(room.as_str()));
        result.err().map(|error| Self::failed(&error)).unwrap_or_default()
    }

    pub fn add(&self, id: String, room: String, text: String) -> String {
        let row = BehaviorValue::object([("id", BehaviorValue::from(id)), ("room", BehaviorValue::from(room)), ("text", BehaviorValue::from(text))]);
        let result = self.behaviors.repository::<MessageEntity>().and_then(|messages| messages.save(row));
        result.err().map(|error| Self::failed(&error)).unwrap_or_default()
    }

    pub fn read(&self, room: String) -> String {
        match self.message_store.find_by_room(BehaviorValue::from(room)) {
            Ok(rows) => Self::ids(&rows),
            Err(error) => Self::failed(&error),
        }
    }

    #[OnEvent]
    pub fn hear_loaded(&mut self, loaded: StorePartitionLoaded) {
        self.heard = format!("{}/{}/{}", loaded.table, loaded.partition, loaded.rows);
    }
}
"#,
        ),
    ];

    pub(crate) fn runtime_files(root: &Path) {
        let files = [
            ("tac-storage.js", include_str!("../template/tac-storage.js")),
            ("tac-web.js", include_str!("../template/tac-web.js")),
            ("memory-indexeddb.js", include_str!("memory_indexeddb.js")),
            (
                "application-events.mjs",
                concat!(
                    include_str!("../template/application-events.js"),
                    "\nexport default TacApplicationEvents\n"
                ),
            ),
        ];
        for (name, text) in files {
            std::fs::write(root.join(name), text).expect("runtime file");
        }
    }

    pub(crate) fn assert_report(report: &serde_json::Value) {
        let expected = serde_json::json!({
            "before": "error:storage_not_loaded",
            "openBefore": false,
            "heard": "messages/r1/3",
            "after": "m1,m2,m4",
            "openAfter": true,
            "closed": "error:storage_not_loaded",
            "cold": ["m9"],
        });
        assert_eq!(report, &expected, "{report}");
    }

    fn written(root: &Path, files: &[(&str, &str)]) -> PathBuf {
        let mut service = PathBuf::new();
        for (relative, text) in files {
            service = write_service(root, relative, text);
        }
        service
    }
}

#[tokio::test]
async fn a_rust_service_loads_one_partition_and_reads_it_from_memory() {
    if !web_assembly_target_installed() || !node_available() {
        eprintln!("skipping: the wasm32 Rust target or node is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = StorePartitionProject::written(root.path(), &StorePartitionProject::RUST_FILES);
    StorePartitionProject::runtime_files(root.path());
    build_and_publish(
        root.path(),
        "chat",
        "ChatService",
        WebLanguage::Rust,
        service,
    )
    .await;
    let report = run_driver(root.path(), StorePartitionProject::DRIVER);
    StorePartitionProject::assert_report(&report);
}
