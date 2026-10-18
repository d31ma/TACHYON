#![allow(clippy::expect_used)]

use super::WebEngine;
use std::time::Duration;

struct TaskBridgeFixture;

impl TaskBridgeFixture {
    const BOUNDARY: &'static str = r"
/** @type {string[]} */
const published = []
/** @type {Array<{request: Record<string, unknown>, resolve: (answer: string) => void}>} */
const pending = []
/** @type {Map<string, number>} */
const counts = new Map()
/** @param {object | null} target @param {() => unknown} work @returns {unknown} */
const calling = (target, work) => work()
/** @param {string} text @returns {string} */
const transport = (text) => {
  const request = JSON.parse(text)
  if (request.op === 'init') {
    counts.set(request.owner, 0)
    return JSON.stringify({ value: { fields: ['count'], methods: ['read', 'load'], asyncMethods: ['load'], listeners: [] } })
  }
  if (request.op === 'dispose') { counts.delete(request.owner); return JSON.stringify({ value: null }) }
  if (request.op === 'set') { counts.set(request.owner, request.value); return JSON.stringify({ value: null }) }
  return JSON.stringify({ value: counts.get(request.owner) })
}
const companion = {
  /** @param {string} text @returns {Promise<string>} */
  InvokeAsync(text) {
    const request = JSON.parse(text)
    return new Promise((resolve) => pending.push({ request, resolve }))
  },
}
";

    const CHECKS: &'static str = r"
class TaskBridgeChecks {
  /** @returns {Promise<void>} */
  static async run() {
    let changes = 0
    const target = { __tachyonStoredChanged() { changes++ } }
    const other = {}
    const plan = { listeners: [], topics: {} }
    activate(target, 'delay', {}, plan)
    activate(other, 'delay', {}, { listeners: [], topics: {} })
    target.count = 7
    if (target.read() !== 7 || other.read() !== 0) throw new Error('sync member contract changed')
    changes = 0
    const old = target.load('obsolete')
    if (!(old instanceof Promise) || pending.length !== 1) throw new Error('Task was replayed or not a Promise')
    const oldOwner = pending[0].request.owner
    const retained = target.load
    target.__tachyonDispose()
    let refused = false
    try { await retained('retired') } catch (error) { refused = error.code === 'CANCELLED' }
    if (!refused || pending.length !== 1) throw new Error('retired method invoked the transport')
    activate(target, 'delay', {}, { listeners: [], topics: {} })
    pending.shift().resolve(JSON.stringify({ value: { message: 'obsolete' } }))
    refused = false
    try { await old } catch (error) { refused = error.code === 'CANCELLED' }
    if (!refused || changes !== 0) throw new Error('retired Promise applied or notified a remounted target')
    const replacement = target.load('replacement')
    const otherPending = other.load('independent')
    if (pending.length !== 2 || pending[0].request.owner === oldOwner || pending[0].request.owner === pending[1].request.owner) throw new Error('activation identities were reused or shared')
    pending.shift().resolve(JSON.stringify({ value: { message: 'replacement' } }))
    if ((await replacement).message !== 'replacement' || changes !== 1) throw new Error('Task result did not decode once')
    other.__tachyonDispose()
    pending.shift().resolve(JSON.stringify({ error: 'cancelled', code: 'CANCELLED' }))
    refused = false
    try { await otherPending } catch (error) { refused = error.code === 'CANCELLED' }
    if (!refused || changes !== 1) throw new Error('disposed owner error lost its code or notified another target')
    target.__tachyonDispose()
    console.log('PASS Task Promise bridge unit')
  }
}
await TaskBridgeChecks.run()
";

    fn run() -> String {
        let (_, task_engine) = WebEngine::CSHARP_ENGINE
            .split_once("const transportAsync")
            .expect("distinct browser Task transport");
        let source = format!(
            "{}\nconst transportAsync{}\n{}\n{}",
            Self::BOUNDARY,
            task_engine,
            WebEngine::SHARED_TAIL,
            Self::CHECKS,
        );
        let root = tempfile::tempdir().expect("bridge source fixture");
        let script = root.path().join("TaskBridgeChecks.mjs");
        std::fs::write(&script, source).expect("actual emitted Task bridge");
        let mut command = crate::node_driver::NodeDriver::command();
        command.arg(script);
        let output =
            crate::node_driver::NodeDriver::output_within(&mut command, Duration::from_secs(20))
                .expect("supervised bridge unit");
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
fn dotnet_task_bridge_settles_once_and_rejects_retired_target_delivery() {
    assert!(TaskBridgeFixture::run().contains("PASS Task Promise bridge unit"));
}
