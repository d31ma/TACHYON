#![allow(clippy::expect_used)]

use super::WebLanguage;
use super::store_partition_tests::StorePartitionProject;
use super::tests::{build_and_publish, node_available, run_driver, write_service};
use std::path::{Path, PathBuf};

struct PythonStorePartition;

impl PythonStorePartition {
    const MANIFEST: &'static str =
        "{\"tac\":{\"python\":{\"runtime\":\"pyodide-314.0.7\",\"profile\":\"offline\"}}}\n";

    const DRIVER: &'static str = r"import { readFile, writeFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
const print = console.log
console.log = () => {}
console.info = () => {}
globalThis.fetch = async (url) => new Response(await readFile(fileURLToPath(url)))
globalThis.addEventListener = () => {}
globalThis.removeEventListener = () => {}
const engine = await readFile('./tachyon-python-services.js', 'utf8')
await writeFile('./tachyon-python-services.js', engine.replace(/(new URL\('(?!pyodide\.mjs)[^']+', root\)|root)\.href/g, `process.getBuiltinModule('node:url').fileURLToPath($1)`))
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
const before = await chat.read('r1')
const openBefore = await chat.isOpen('r1')
await Promise.all([chat.open('r1'), chat.add('m4', 'r1', 'late'), chat.add('m9', 'r3', 'cold')])
await settle()
const heard = chat.heard
const after = await chat.read('r1')
const openAfter = await chat.isOpen('r1')
await chat.close('r1')
const closed = await chat.read('r1')
await settle()
const cold = MemoryIndexedDb.rows('tachyon-store', 'messages').filter((row) => row.room === 'r3').map((row) => row.id)
print(JSON.stringify({ before, openBefore, heard, after, openAfter, closed, cold }))
process.exit(0)
";

    const FILES: [(&'static str, &'static str); 4] = [
        (
            "client/setups/ChatSetup.py",
            "@setup\nclass ChatSetup:\n    @staticmethod\n    def configure(execution):\n        return {}\n",
        ),
        (
            "client/entities/MessageEntity.py",
            "from typing import Annotated\n\n@entity({\"adapter\":\"indexeddb\",\"partition\":\"room\"})\n@table(\"messages\")\nclass MessageEntity:\n    id: Annotated[str, id()] = ''\n    room: str = ''\n    text: str = ''\n",
        ),
        (
            "client/stores/MessageStore.py",
            "from typing import Protocol\n\n@store(MessageEntity, {\"fields\":[\"id\",\"room\",\"text\"],\"defaultSize\":20,\"maxSize\":100,\"setup\":\"ChatSetup\"})\nclass MessageStore(Protocol):\n    async def find_by_room(self, room: str) -> list[dict[str, str]]: ...\n",
        ),
        (
            "client/services/ChatService.py",
            r"from typing import Annotated

@service
class ChatService:
    message_store: Annotated[MessageStore, needs]
    heard: str = ''

    @on_start
    async def start(self) -> None:
        self.behaviors = await Behaviors.create(ChatSetup, {'partition': 'chat', 'signal': BehaviorSignal()})

    async def open(self, room: str) -> str:
        try:
            await self.behaviors.repository(MessageEntity).load(room)
        except BehaviorError as error:
            return 'error:' + error.code
        return ''

    async def is_open(self, room: str) -> bool:
        try:
            return await self.behaviors.repository(MessageEntity).loaded(room)
        except BehaviorError:
            return False

    async def close(self, room: str) -> str:
        try:
            await self.behaviors.repository(MessageEntity).unload(room)
        except BehaviorError as error:
            return 'error:' + error.code
        return ''

    async def add(self, id: str, room: str, text: str) -> str:
        try:
            await self.behaviors.repository(MessageEntity).save({'id': id, 'room': room, 'text': text})
        except BehaviorError as error:
            return 'error:' + error.code
        return ''

    async def read(self, room: str) -> str:
        try:
            rows = await self.message_store.find_by_room(room)
        except BehaviorError as error:
            return 'error:' + error.code
        return ','.join(sorted(row['id'] for row in rows))

    @on_event
    def hear_loaded(self, loaded: StorePartitionLoaded) -> None:
        self.heard = f'{loaded.table}/{loaded.partition}/{int(loaded.rows)}'
",
        ),
    ];

    fn skip_reason() -> Option<String> {
        if !node_available() {
            return Some(String::from("node is unavailable"));
        }
        let assets = crate::tac_python::RuntimeAssets::cached_root();
        let missing = crate::tac_python::RuntimeAssets::read(&assets).is_err();
        assert!(
            !missing || std::env::var_os("CI").is_none(),
            "CI installs the pinned Pyodide with scripts/ci/install-toolchains.mjs, so the Python Tac tests must run there"
        );
        missing.then(|| String::from("the pinned Pyodide runtime is not in the Tachyon cache"))
    }

    fn written(root: &Path) -> PathBuf {
        write_service(root, "client/manifest.json", Self::MANIFEST);
        let mut service = PathBuf::new();
        for (relative, text) in Self::FILES {
            service = write_service(root, relative, text);
        }
        service
    }
}

#[tokio::test]
async fn a_python_service_loads_one_partition_and_reads_it_from_memory() {
    if let Some(reason) = PythonStorePartition::skip_reason() {
        eprintln!("skipping: {reason}");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    let service = PythonStorePartition::written(root.path());
    StorePartitionProject::runtime_files(root.path());
    build_and_publish(
        root.path(),
        "chat",
        "ChatService",
        WebLanguage::Python,
        service,
    )
    .await;
    let report = run_driver(root.path(), PythonStorePartition::DRIVER);
    StorePartitionProject::assert_report(&report);
}
