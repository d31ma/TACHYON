#![allow(clippy::expect_used)]

use std::time::Duration;

struct FocusFixture;

impl FocusFixture {
    const PAGE: &'static str = r"
const listeners = {}
globalThis.document = {
  activeElement: null,
  addEventListener: (type, listener) => { (listeners[type] ??= []).push(listener) },
}
const fire = (type, event) => { for (const listener of listeners[type] ?? []) listener(event) }
class FakeElement {
  constructor(tag, attributes = {}, children = []) {
    this.tag = tag
    this.attributes = attributes
    this.children = children
    this.parentNode = null
    this.isConnected = true
    this.disabled = false
    this.tabIndex = 'tabindex' in attributes ? Number(attributes.tabindex) : ['button', 'input'].includes(tag) ? 0 : -1
    for (const child of children) child.parentNode = this
  }
  getAttribute(name) { return this.attributes[name] ?? null }
  focus() {
    if (!this.isConnected || (this.tabIndex < 0 && !('tabindex' in this.attributes))) return
    const before = document.activeElement
    document.activeElement = this
    if (before !== this) fire('focusin', { target: this, composedPath: () => [this] })
  }
  closest() { return null }
  getClientRects() { return [1] }
  all() { return this.children.flatMap((child) => [child, ...child.all()]) }
  add(child) { child.parentNode = this; this.children.push(child) }
  querySelectorAll(selector) {
    if (selector === '[data-focus]') return this.all().filter((element) => 'data-focus' in element.attributes)
    return this.all().filter((element) => ['button', 'input'].includes(element.tag) || 'tabindex' in element.attributes)
  }
}
";

    const CHECKS: &'static str = r"
const equal = (expected, actual, label) => { if (expected !== actual) throw new Error(`${label}: expected ${expected}, got ${actual}`) }
const web = globalThis[Symbol.for('tachyon.tac.web')]
const sent = []
TacWebApis.events = { send: (owner, name, value) => sent.push({ owner, name, value }) }
const owner = { name: 'dialog service' }
const stranger = { name: 'unused service' }
const field = new FakeElement('input', { 'data-focus': 'name' })
const middle = new FakeElement('input', {})
const save = new FakeElement('button', { 'data-focus': 'save' })
const dialog = new FakeElement('div', { 'data-focus': 'dialog', tabindex: '-1' }, [field, middle, save])
const opener = new FakeElement('button', { 'data-focus': 'open' })
const outside = new FakeElement('button', { 'data-focus': 'outside' })
const host = new FakeElement('tachyon-component', {}, [opener, outside, dialog])
const records = new Map(host.all().map((element) => [element, 'component']))
let renders = 0
TacFocus.connect({
  render: () => { renders += 1; return Promise.resolve() },
  views: (caller) => caller === owner ? [{ owner: 'component', host }] : [],
  owner: (element) => records.get(element),
})
const ask = (caller, call, args) => JSON.parse(TacWebApis.calling(caller, () => web.host(JSON.stringify({ api: 'focus', call, args }))))
const tab = (backward) => {
  const event = { key: 'Tab', shiftKey: backward, prevented: false, preventDefault() { this.prevented = true } }
  fire('keydown', event)
  return event.prevented
}
const failure = () => { const last = sent.pop(); equal(owner, last?.owner, 'failure owner'); equal('focus', last.value.api, 'failure api'); return last.value.message }

equal('A focus name', ask(owner, 'move', ['1bad']).error.slice(0, 12), 'bad name')
equal('A focus name', ask(owner, 'trap', [7]).error.slice(0, 12), 'name type')
equal('Focus needs the service that calls it. Call it from a service method.', JSON.parse(web.host(JSON.stringify({ api: 'focus', call: 'move', args: ['save'] }))).error, 'no caller')
equal('Focus works only for a service that a shown component uses.', ask(stranger, 'move', ['save']).error, 'no view')

opener.focus()
equal(null, ask(owner, 'trap', ['dialog']).value, 'trap answer')
equal(opener, document.activeElement, 'trap waits for the render')
await Promise.resolve()
await Promise.resolve()
equal(true, renders > 0, 'trap asks for a render')
TacFocus.settle()
equal(field, document.activeElement, 'trap focuses the first stop')
equal(false, tab(false), 'tab inside is left to the browser')
save.focus()
equal(true, tab(false), 'tab at the end is caught')
equal(field, document.activeElement, 'tab wraps to the first stop')
equal(true, tab(true), 'shift tab at the start is caught')
equal(save, document.activeElement, 'shift tab wraps to the last stop')
outside.focus()
equal(field, document.activeElement, 'focus outside the trap comes back')
document.activeElement = outside
equal(true, tab(false), 'tab from outside the trap is caught')
equal(field, document.activeElement, 'tab from outside goes to the first stop')

ask(owner, 'move', ['save'])
TacFocus.settle()
equal(save, document.activeElement, 'move focuses the named element')
TacFocus.owned(owner).move('name')
TacFocus.settle()
equal(field, document.activeElement, 'the owned resource moves focus')

ask(owner, 'release', [])
TacFocus.settle()
equal(opener, document.activeElement, 'release returns focus to the opener')
equal(false, tab(false), 'no trap leaves tab alone')
equal(0, sent.length, 'no failure so far')

ask(owner, 'move', ['missing'])
TacFocus.settle()
equal('No element with data-focus=\x22missing\x22 is shown by a view of this service.', failure(), 'missing target')
ask(owner, 'release', [])
TacFocus.settle()
equal('This service has no focus trap to release.', failure(), 'nothing to release')
host.add(new FakeElement('button', { 'data-focus': 'twin' }))
host.add(new FakeElement('button', { 'data-focus': 'twin' }))
for (const element of host.all()) records.set(element, 'component')
ask(owner, 'move', ['twin'])
TacFocus.settle()
equal('More than one element has', failure().slice(0, 25), 'two targets')
const label = new FakeElement('span', { 'data-focus': 'label' })
host.add(label)
records.set(label, 'component')
ask(owner, 'move', ['label'])
TacFocus.settle()
equal('cannot take focus', failure().split('\x22label\x22 ')[1]?.slice(0, 17), 'not focusable')
const foreign = new FakeElement('button', { 'data-focus': 'foreign' })
host.add(foreign)
ask(owner, 'move', ['foreign'])
TacFocus.settle()
equal('No element with', failure().slice(0, 15), 'an element of another owner')
equal(sent.length, 0, 'failures go to the caller only')

opener.focus()
ask(owner, 'trap', ['dialog'])
TacFocus.settle()
opener.isConnected = false
host.children.splice(host.children.indexOf(opener), 1)
const newOpener = new FakeElement('button', { 'data-focus': 'open' })
host.add(newOpener)
records.set(newOpener, 'component')
ask(owner, 'release', [])
TacFocus.settle()
equal(newOpener, document.activeElement, 'release finds a replaced opener by name')

for (let count = 0; count < TacFocus.mostWaiting; count += 1) equal(null, ask(owner, 'move', ['save']).value, 'queued')
equal('Focus keeps at most 16 requests until the next render.', ask(owner, 'move', ['save']).error, 'queue limit')
TacFocus.settle()
equal(save, document.activeElement, 'the queue drains in order')
console.log('focus-fixture-passed')
";

    const GO_SERVICE: &'static str = "import \"fmt\"\n\n// @Service\ntype StampService struct {\n\t// @Needs\n\tfocus Focus\n}\n\n\
         func (stamp *StampService) Open() string {\n\treturn fmt.Sprint(stamp.focus.Trap(\"dialog\"))\n}\n\n\
         func (stamp *StampService) Pick() string {\n\treturn fmt.Sprint(stamp.focus.Move(\"save\"))\n}\n\n\
         func (stamp *StampService) Close() string {\n\treturn fmt.Sprint(stamp.focus.Release())\n}\n\n\
         func (stamp *StampService) Bad() string {\n\treturn fmt.Sprint(stamp.focus.Move(\"1bad\"))\n}\n";

    const GO_CHECKS: &'static str = r"
const equal = (expected, actual, label) => { if (expected !== actual) throw new Error(`${label}: expected ${expected}, got ${actual}`) }
const field = new FakeElement('input', {})
const save = new FakeElement('button', { 'data-focus': 'save' })
const dialog = new FakeElement('div', { 'data-focus': 'dialog' }, [field, save])
const opener = new FakeElement('button', {})
const host = new FakeElement('tachyon-component', {}, [opener, dialog])
globalThis.fetch = async (url) => new Response(await readFile(url))
const { default: StampService } = await import('./StampService.js')
const stamp = new StampService({})
TacFocus.connect({
  render: () => Promise.resolve(),
  views: (caller) => caller === stamp ? [{ owner: 'component', host }] : [],
  owner: (element) => host.all().includes(element) ? 'component' : null,
})
equal(true, stamp.bad().startsWith('A focus name'), 'a bad name is a Go error')
opener.focus()
equal('<nil>', stamp.open(), 'trap')
TacFocus.settle()
equal(field, document.activeElement, 'the Go trap focuses the first stop')
equal('<nil>', stamp.pick(), 'move')
TacFocus.settle()
equal(save, document.activeElement, 'the Go move focuses save')
equal('<nil>', stamp.close(), 'release')
TacFocus.settle()
equal(opener, document.activeElement, 'the Go release returns focus')
console.log('focus-go-passed')
process.exit(0)
";

    fn run() -> String {
        let program = [
            Self::PAGE,
            include_str!("../template/tac-focus.js"),
            include_str!("../template/tac-web.js"),
            Self::CHECKS,
        ]
        .join("\n");
        let root = tempfile::tempdir().expect("fixture");
        Self::execute(root.path(), &program)
    }

    async fn run_go() -> String {
        let root = tempfile::tempdir().expect("fixture");
        let path = root.path().join("client/services/StampService.go");
        std::fs::create_dir_all(path.parent().expect("parent")).expect("services");
        std::fs::write(&path, Self::GO_SERVICE).expect("service");
        let services = [(
            String::from("stamp"),
            String::from("StampService"),
            super::WebLanguage::Go,
            path,
        )];
        let jobs = super::BuildJobs::new(1);
        let published =
            super::build_all(root.path(), &services, &jobs, crate::BuildProfile::Release)
                .await
                .expect("build");
        for (name, bytes) in published.files {
            let file = root.path().join(name);
            std::fs::create_dir_all(file.parent().expect("parent")).expect("folder");
            std::fs::write(file, bytes).expect("module");
        }
        let program = [
            "import { readFile } from 'node:fs/promises'",
            Self::PAGE,
            include_str!("../template/tac-focus.js"),
            include_str!("../template/tac-web.js"),
            Self::GO_CHECKS,
        ]
        .join("\n");
        Self::execute(root.path(), &program)
    }

    fn go_available() -> bool {
        std::process::Command::new("go")
            .arg("version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    fn execute(root: &std::path::Path, program: &str) -> String {
        let script = root.join("focus.mjs");
        std::fs::write(&script, program).expect("program");
        let output = crate::node_driver::NodeDriver::output_within(
            crate::node_driver::NodeDriver::command()
                .arg(script)
                .current_dir(root),
            Duration::from_mins(2),
        )
        .expect("supervised Node");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

#[test]
fn focus_moves_traps_tab_and_returns_focus_after_the_render() {
    assert!(FocusFixture::run().contains("focus-fixture-passed"));
}

#[tokio::test]
async fn a_go_service_moves_traps_and_releases_focus_through_the_page() {
    if !crate::node_driver::NodeDriver::available() || !FocusFixture::go_available() {
        eprintln!("skipping: node or go is unavailable");
        return;
    }
    assert!(FocusFixture::run_go().await.contains("focus-go-passed"));
}
