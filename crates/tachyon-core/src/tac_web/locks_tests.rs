use super::{WebApis, WebLanguage};
use std::path::Path;

pub(super) struct LocksTests;

impl LocksTests {
    const MODULE: &str = include_str!("../template/tac-locks.js");

    const SERVICE_DRIVER: &str = r";(0, eval)(`${await readFile('./tac-web.js', 'utf8')}\nglobalThis.TacWebApis = TacWebApis`)
;(0, eval)(await readFile('./tac-locks.js', 'utf8'))
const { default: TacApplicationEvents } = await import('./application-events.mjs')
const { default: StampService, __tachyon_event_plan__: plan } = await import('./StampService.js')
__GRANT_STUBS__
grants.content = 'locks'
const asked = []
const freed = []
navigator.locks = {
  request: (name, options, callback) => { asked.push(name); return Promise.resolve(callback()).then(() => { freed.push(name) }) },
}
const settle = async () => { for (let round = 0; round < 10; round += 1) await new Promise((resolve) => setTimeout(resolve, 0)) }
const bus = new TacApplicationEvents({ changed: () => {}, failed: () => { throw new Error('Event delivery failed') } })
TacWebApis.events = { publish: (name, value) => bus.publish(name, value), send: (owner, name, value) => bus.send(owner, name, value) }
const stamp = new StampService({})
bus.attach(stamp, plan.types[0], { applicationEventPublisher: null })
await bus.initialize(stamp)
stamp.take()
await settle()
const held = stamp.held
stamp.give()
await settle()
console.log(JSON.stringify({ held, asked, freed }))
process.exit(0)
";

    const RUST: &str = r#"#[Service]
#[derive(Default)]
struct StampService {
    locks: Locks,
    held: String,
}

impl StampService {
    pub fn take(&self) {
        let _ = self.locks.request("outbox");
    }

    pub fn give(&self) {
        let _ = self.locks.release("outbox");
    }

    #[OnEvent]
    pub fn hear_granted(&mut self, granted: LockGranted) {
        self.held = granted.name;
    }
}
"#;

    pub(super) fn assert_held(root: &Path, fetches_files: bool) {
        WebApis::write(
            root,
            "application-events.mjs",
            WebApis::EVENTS_RUNTIME.as_bytes(),
        );
        WebApis::write(root, "tac-locks.js", Self::MODULE.as_bytes());
        let held = Self::SERVICE_DRIVER.replace("__GRANT_STUBS__", WebApis::GRANT_STUBS);
        let driver = if fetches_files {
            format!("{}{held}", WebApis::FILE_FETCH)
        } else {
            held
        };
        let report = WebApis::run(root, &driver);
        assert_eq!(
            report,
            serde_json::json!({
                "held": "outbox",
                "asked": ["tachyon:outbox"],
                "freed": ["tachyon:outbox"]
            })
        );
    }
}

#[tokio::test]
async fn a_rust_web_service_takes_and_releases_a_lock() {
    if !WebApis::node_available() || !WebApis::rust_web_target() {
        eprintln!("skipping: node or the wasm32 Rust target is unavailable");
        return;
    }
    let root = tempfile::tempdir().expect("project");
    WebApis::publish(root.path(), WebLanguage::Rust, "rs", LocksTests::RUST).await;
    LocksTests::assert_held(root.path(), false);
}
