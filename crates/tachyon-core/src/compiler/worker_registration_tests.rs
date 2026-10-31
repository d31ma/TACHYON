use super::WebCompiler;
use crate::node_driver::NodeDriver;

struct WorkerRegistrationFixture;

impl WorkerRegistrationFixture {
    const HARNESS: &'static str = r"import { readFileSync } from 'node:fs'
const registration = readFileSync('register-sw.js', 'utf8')
const scenarios = {
  replaced: { controller: true },
  first: { controller: false },
  edited: { controller: true, edit: true },
  recent: { controller: true, stamp: Date.now() },
  stale: { controller: true, stamp: Date.now() - 60000 },
  offline: { controller: true, online: false },
}
const results = []
let load = 0
for (const [name, setup] of Object.entries(scenarios)) {
  let reloads = 0, registered = 0
  const workerListeners = new Map(), pageListeners = new Map()
  const storage = new Map(setup.stamp ? [['tachyon:worker-reload', String(setup.stamp)]] : [])
  globalThis.addEventListener = (type, listener) => pageListeners.set(type, listener)
  globalThis.location = { hostname: 'shop.example', reload() { reloads++ } }
  Object.defineProperty(globalThis, 'sessionStorage', { configurable: true, value: {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, value),
  } })
  Object.defineProperty(globalThis, 'navigator', { configurable: true, value: {
    onLine: setup.online ?? true,
    serviceWorker: {
      controller: setup.controller ? {} : null,
      addEventListener(type, listener, options) { workerListeners.set(type, { listener, once: options?.once === true }) },
      register() { registered++; return Promise.resolve({}) },
    },
  } })
  await import(`data:text/javascript,${encodeURIComponent(`${registration}\n// load ${load++}`)}`)
  if (setup.edit) pageListeners.get('input')()
  for (let change = 0; change < 2; change++) {
    const entry = workerListeners.get('controllerchange')
    if (!entry) break
    if (entry.once) workerListeners.delete('controllerchange')
    entry.listener()
  }
  results.push(`${name}=${reloads}/${registered}`)
}
console.log(results.join(' '))
";

    fn run() -> String {
        let folder = tempfile::tempdir().unwrap_or_else(|error| panic!("harness folder: {error}"));
        std::fs::write(
            folder.path().join("register-sw.js"),
            WebCompiler::SERVICE_WORKER_REGISTRATION,
        )
        .unwrap_or_else(|error| panic!("registration script: {error}"));
        let mut command = NodeDriver::command();
        command
            .args(["--input-type=module", "--eval", Self::HARNESS])
            .current_dir(folder.path());
        let output = NodeDriver::output(&mut command)
            .unwrap_or_else(|error| panic!("node harness: {error}"));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    }
}

#[test]
fn a_page_an_older_worker_served_reloads_once_when_the_new_worker_takes_control() {
    assert_eq!(
        WorkerRegistrationFixture::run(),
        "replaced=1/1 first=0/1 edited=0/1 recent=0/1 stale=1/1 offline=0/1"
    );
}
