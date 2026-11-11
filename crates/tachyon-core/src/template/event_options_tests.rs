use crate::node_driver::NodeDriver;
use serde_json::{Value, json};

struct EventOptionsDriver;

impl EventOptionsDriver {
    const DRIVER: &'static str = r"import { readFile } from 'node:fs/promises'
globalThis.document = new EventTarget()
globalThis.window = new EventTarget()
;(0, eval)(`${await readFile('./tac-event-options.js', 'utf8')}\nglobalThis.TacEventOptions = TacEventOptions`)
const calls = []
const handle = (name) => async () => { calls.push(name) }
const press = (binding, fields) => {
  const event = { preventDefault() { this.prevented = true }, stopPropagation() { this.stopped = true }, ...fields }
  const before = calls.length
  TacEventOptions.listener(binding, handle('x'), { element: null })(event)
  return { handled: calls.length > before, prevented: Boolean(event.prevented), stopped: Boolean(event.stopped) }
}
const escape = { key: 'Escape', modifierKeys: [], prevent: true, stop: true }
const sendKey = { key: 'Enter', modifierKeys: ['shiftKey'] }
const keys = {
  escape: press(escape, { key: 'Escape' }),
  shiftEscape: press(escape, { key: 'Escape', shiftKey: true }),
  enterOnEscape: press(escape, { key: 'Enter' }),
  composing: press(escape, { key: 'Escape', isComposing: true }),
  shiftEnter: press(sendKey, { key: 'Enter', shiftKey: true }),
  enter: press(sendKey, { key: 'Enter' }),
  ctrlShiftEnter: press(sendKey, { key: 'Enter', shiftKey: true, ctrlKey: true }),
  shiftClick: press({ modifierKeys: ['shiftKey'] }, { shiftKey: true }),
  plainClickOnShift: press({ modifierKeys: ['shiftKey'] }, {}),
  plain: press({ prevent: true }, { key: 'q', altKey: true }),
}
const element = { isConnected: true }
const inner = { isConnected: true }
const outsideBinding = { outside: true }
const click = (path) => {
  const before = calls.length
  TacEventOptions.listener(outsideBinding, handle('outside'), { element })({ composedPath: () => path })
  return calls.length > before
}
const outside = { elsewhere: click([document]), inside: click([inner, element, document]), self: click([element, document]) }
element.isConnected = false
outside.detached = click([document])

const heard = []
const listen = (name) => (event) => { heard.push(name) }
const render = (entries) => {
  TacEventOptions.begin()
  for (const [key, type, binding, listener] of entries) TacEventOptions.defer(key, type, binding, listener)
  TacEventOptions.commit()
}
const early = new Event('keydown')
await new Promise((resolve) => setTimeout(resolve, 5))
render([['body.0|data-tac-on-document__keydown', 'keydown', { listen: 'document' }, listen('first')]])
document.dispatchEvent(early)
const lifecycle = { inFlight: [...heard] }
await new Promise((resolve) => setTimeout(resolve, 5))
document.dispatchEvent(new Event('keydown'))
lifecycle.attached = [...heard]
heard.length = 0
render([['body.0|data-tac-on-document__keydown', 'keydown', { listen: 'document' }, listen('second')], ['body.1|data-tac-on-window__resize', 'resize', { listen: 'window' }, listen('resize')]])
document.dispatchEvent(new Event('keydown'))
window.dispatchEvent(new Event('resize'))
document.dispatchEvent(new Event('resize'))
lifecycle.kept = [...heard]
heard.length = 0
render([])
document.dispatchEvent(new Event('keydown'))
window.dispatchEvent(new Event('resize'))
lifecycle.removed = [...heard]

TacEventOptions.begin()
const fresh = {}
const live = {}
const holder = TacEventOptions.holder(fresh)
TacEventOptions.moved(live, fresh)
TacEventOptions.moved(live, {})
const moved = { live: holder.element === live, same: TacEventOptions.holder(fresh) === holder }
TacEventOptions.commit()
await new Promise((resolve) => setTimeout(resolve, 0))
console.log(JSON.stringify({ keys, outside, lifecycle, moved, above: [TacEventOptions.listensAbove({}), TacEventOptions.listensAbove({ listen: 'window' }), TacEventOptions.listensAbove({ outside: true })] }))
process.exit(0)
";

    fn run() -> Value {
        let root = tempfile::tempdir().expect("root");
        for (name, source) in [
            ("tac-event-options.js", include_str!("tac-event-options.js")),
            ("drive.mjs", Self::DRIVER),
        ] {
            std::fs::write(root.path().join(name), source).expect("write driver file");
        }
        let mut command = NodeDriver::command();
        command.args(["drive.mjs"]).current_dir(root.path());
        let output = NodeDriver::output(&mut command).expect("node drive.mjs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{stdout}{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_str(stdout.lines().last().unwrap_or_default()).expect("report")
    }

    fn outcome(handled: bool, prevented: bool, stopped: bool) -> Value {
        json!({ "handled": handled, "prevented": prevented, "stopped": stopped })
    }
}

#[test]
fn key_filters_match_the_key_and_the_exact_modifier_keys() {
    let report = EventOptionsDriver::run();
    let keys = &report["keys"];
    assert_eq!(
        keys["escape"],
        EventOptionsDriver::outcome(true, true, true)
    );
    for refused in ["shiftEscape", "enterOnEscape", "composing"] {
        assert_eq!(
            keys[refused],
            EventOptionsDriver::outcome(false, false, false),
            "{refused}"
        );
    }
    assert_eq!(keys["shiftEnter"]["handled"], true);
    assert_eq!(keys["enter"]["handled"], false);
    assert_eq!(keys["ctrlShiftEnter"]["handled"], false);
    assert_eq!(keys["shiftClick"]["handled"], true);
    assert_eq!(keys["plainClickOnShift"]["handled"], false);
    assert_eq!(
        keys["plain"],
        EventOptionsDriver::outcome(true, true, false)
    );
}

#[test]
fn outside_listeners_fire_only_for_events_outside_a_connected_element() {
    let report = EventOptionsDriver::run();
    assert_eq!(
        report["outside"],
        json!({ "elsewhere": true, "inside": false, "self": false, "detached": false })
    );
    assert_eq!(report["above"], json!([false, true, true]));
}

#[test]
fn document_and_window_listeners_follow_each_render() {
    let report = EventOptionsDriver::run();
    assert_eq!(
        report["lifecycle"],
        json!({
            "inFlight": [],
            "attached": ["first"],
            "kept": ["second", "resize"],
            "removed": [],
        })
    );
}

#[test]
fn moved_elements_keep_their_outside_holder() {
    let report = EventOptionsDriver::run();
    assert_eq!(report["moved"], json!({ "live": true, "same": true }));
}
