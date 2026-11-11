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

    const GO: &'static str = "import \"fmt\"\n\n// @Service\ntype FocusService struct {\n\t// @Needs\n\tfocus Focus\n}\n\n\
         func (service *FocusService) Open() string {\n\treturn fmt.Sprint(service.focus.Trap(\"dialog\"))\n}\n\n\
         func (service *FocusService) Pick() string {\n\treturn fmt.Sprint(service.focus.Move(\"save\"))\n}\n\n\
         func (service *FocusService) Shut() string {\n\treturn fmt.Sprint(service.focus.Release())\n}\n\n\
         func (service *FocusService) Bad() string {\n\treturn fmt.Sprint(service.focus.Move(\"1bad\"))\n}\n";

    const RUST: &'static str = "#[Service]\n#[derive(Default)]\nstruct FocusService {\n    focus: Focus,\n}\n\n\
         impl FocusService {\n    \
         pub fn open(&self) -> String {\n        self.focus.trap(\"dialog\").err().unwrap_or_default()\n    }\n\n    \
         pub fn pick(&self) -> String {\n        self.focus.move_to(\"save\").err().unwrap_or_default()\n    }\n\n    \
         pub fn shut(&self) -> String {\n        self.focus.release().err().unwrap_or_default()\n    }\n}\n";

    const CSHARP: &'static str = "[Service]\npublic class FocusService\n{\n    \
         private readonly Focus focus = new();\n\n    \
         public void Open() => focus.Trap(\"dialog\");\n\n    \
         public void Pick() => focus.Move(\"save\");\n\n    \
         public void Shut() => focus.Release();\n}\n";

    const FSHARP: &'static str = "[<Service>]\ntype FocusService() =\n    let focus = Focus()\n\n    \
         member this.Open() : unit = focus.Trap(\"dialog\")\n\n    \
         member this.Pick() : unit = focus.Move(\"save\")\n\n    \
         member this.Shut() : unit = focus.Release()\n";

    const SWIFT: &'static str = "@Service\nfinal class FocusService {\n    private let focus = Focus()\n\n    \
         func open() throws {\n        try focus.trap(\"dialog\")\n    }\n\n    \
         func pick() throws {\n        try focus.move(\"save\")\n    }\n\n    \
         func shut() throws {\n        try focus.release()\n    }\n}\n";

    const KOTLIN: &'static str = "@Service\nclass FocusService {\n    private val focus = Focus()\n\n    \
         fun open() {\n        focus.trap(\"dialog\")\n    }\n\n    \
         fun pick() {\n        focus.move(\"save\")\n    }\n\n    \
         fun shut() {\n        focus.release()\n    }\n}\n";

    const JAVA: &'static str = "@Service\nclass FocusService {\n    private final Focus focus = new Focus();\n\n    \
         void open() {\n        focus.trap(\"dialog\");\n    }\n\n    \
         void pick() {\n        focus.move(\"save\");\n    }\n\n    \
         void shut() {\n        focus.release();\n    }\n}\n";

    const DART: &'static str = "@Service()\nclass FocusService {\n  @Needs()\n  final Focus focus;\n\n  \
         void open() => focus.trap('dialog');\n\n  \
         void pick() => focus.move('save');\n\n  \
         void shut() => focus.release();\n}\n";

    const RUBY: &'static str = "#[Service]\nclass FocusService\n  \
         def open\n    Focus.new.trap(\"dialog\")\n  end\n\n  \
         def pick\n    Focus.new.move(\"save\")\n  end\n\n  \
         def shut\n    Focus.new.release\n  end\nend\n";

    const LUA: &'static str = "-- @Service\n---@class FocusService\nFocusService = {}\n\n\
         ---@return nil\nfunction FocusService:open()\n  Focus:trap(\"dialog\")\nend\n\n\
         ---@return nil\nfunction FocusService:pick()\n  Focus:move(\"save\")\nend\n\n\
         ---@return nil\nfunction FocusService:shut()\n  Focus:release()\nend\n";

    const CPP: &'static str = "#pragma once\n\nclass [[Service]] FocusService {\npublic:\n    [[Needs]] Focus& focus;\n\n    \
         void open() { focus.trap(\"dialog\"); }\n\n    \
         void pick() { focus.move(\"save\"); }\n\n    \
         void shut() { focus.release(); }\n};\n";

    const SERVICE_CHECKS: &'static str = r"
const equal = (expected, actual, label) => { if (expected !== actual) throw new Error(`${label}: expected ${expected}, got ${actual}`) }
const clean = (answer) => answer === undefined || answer === null || answer === '' || answer === '<nil>'
const field = new FakeElement('input', {})
const save = new FakeElement('button', { 'data-focus': 'save' })
const dialog = new FakeElement('div', { 'data-focus': 'dialog' }, [field, save])
const opener = new FakeElement('button', {})
const host = new FakeElement('tachyon-component', {}, [opener, dialog])
globalThis.fetch = async (url) => new Response(await readFile(url))
const { default: FocusService } = await import('./FocusService.js')
const service = new FocusService({})
TacFocus.connect({
  render: () => Promise.resolve(),
  views: (caller) => caller === service ? [{ owner: 'component', host }] : [],
  owner: (element) => host.all().includes(element) ? 'component' : null,
})
if (typeof service.bad === 'function') equal(true, service.bad().startsWith('A focus name'), 'a bad name fails the call')
opener.focus()
equal(true, clean(await service.open()), 'trap')
TacFocus.settle()
equal(field, document.activeElement, 'the trap focuses the first stop')
equal(true, clean(await service.pick()), 'move')
TacFocus.settle()
equal(save, document.activeElement, 'the move focuses save')
equal(true, clean(await service.shut()), 'release')
TacFocus.settle()
equal(opener, document.activeElement, 'the release returns focus')
console.log('focus-service-passed')
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

    async fn run_service(
        language: super::WebLanguage,
        relative: &str,
        source: &str,
        profile: crate::BuildProfile,
    ) -> String {
        let root = tempfile::tempdir().expect("fixture");
        let path = Self::write(root.path(), relative, source.as_bytes());
        let services = [(
            String::from("focus"),
            String::from("FocusService"),
            language,
            path,
        )];
        let jobs = super::BuildJobs::new(1);
        let published = super::build_all(root.path(), &services, &jobs, profile)
            .await
            .expect("build");
        for (name, bytes) in published.files {
            Self::write(root.path(), &name, &bytes);
        }
        Self::drive(root.path())
    }

    async fn run_dart() -> String {
        let root = tempfile::tempdir().expect("fixture");
        let path = Self::write(
            root.path(),
            "client/services/FocusService.dart",
            Self::DART.as_bytes(),
        );
        let services = [("FocusService", path.as_path())];
        let instances = std::collections::BTreeSet::from([String::from("focus")]);
        let profile = crate::BuildProfile::Release;
        let modules = crate::tac_dart::build_all(root.path(), &services, &instances, profile)
            .await
            .expect("build");
        for (name, bytes) in modules {
            Self::write(root.path(), &name, &bytes);
        }
        Self::write(root.path(), "package.json", b"{\"type\":\"module\"}");
        Self::drive(root.path())
    }

    fn drive(root: &std::path::Path) -> String {
        let program = [
            "import { readFile } from 'node:fs/promises'",
            Self::PAGE,
            include_str!("../template/tac-focus.js"),
            include_str!("../template/tac-web.js"),
            Self::SERVICE_CHECKS,
        ]
        .join("\n");
        Self::execute(root, &program)
    }

    fn write(root: &std::path::Path, relative: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folder");
        std::fs::write(&path, bytes).expect("file");
        path
    }

    fn answers(program: &str, argument: &str) -> bool {
        crate::program_probe::ProgramProbe::answers(program, argument)
            && crate::node_driver::NodeDriver::available()
    }

    fn rust_web_target() -> bool {
        std::process::Command::new("rustc")
            .args(["--print", "target-libdir", "--target"])
            .arg(crate::native::RustCompanion::WEB_TARGET)
            .output()
            .is_ok_and(|output| {
                std::path::Path::new(String::from_utf8_lossy(&output.stdout).trim()).is_dir()
            })
    }

    fn skipped(ready: bool, language: &str) -> bool {
        if !ready {
            eprintln!("skipping: the {language} toolchain or node is unavailable");
        }
        !ready
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
    if FocusFixture::skipped(FocusFixture::answers("go", "version"), "Go") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Go,
        "client/services/FocusService.go",
        FocusFixture::GO,
        crate::BuildProfile::Release,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_rust_service_moves_traps_and_releases_focus_through_the_page() {
    let ready = FocusFixture::rust_web_target() && crate::node_driver::NodeDriver::available();
    if FocusFixture::skipped(ready, "Rust wasm32") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Rust,
        "client/services/FocusService.rs",
        FocusFixture::RUST,
        crate::BuildProfile::Release,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_csharp_service_moves_traps_and_releases_focus_through_the_page() {
    if FocusFixture::skipped(super::fsharp_tests::fsharp_web_ready(), ".NET wasm-tools") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::CSharp,
        "client/services/FocusService.cs",
        FocusFixture::CSHARP,
        crate::BuildProfile::Development,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn an_fsharp_service_moves_traps_and_releases_focus_through_the_page() {
    if FocusFixture::skipped(super::fsharp_tests::fsharp_web_ready(), ".NET wasm-tools") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::CSharp,
        "client/services/FocusService.fs",
        FocusFixture::FSHARP,
        crate::BuildProfile::Release,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_swift_service_moves_traps_and_releases_focus_through_the_page() {
    if FocusFixture::skipped(super::swift_tests::swift_web_ready(), "Swift WebAssembly") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Swift,
        "client/services/FocusService.swift",
        FocusFixture::SWIFT,
        crate::BuildProfile::Development,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_kotlin_service_moves_traps_and_releases_focus_through_the_page() {
    if FocusFixture::skipped(super::kotlin_tests::kotlin_web_ready(), "Kotlin/Wasm") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Kotlin,
        "client/services/FocusService.kt",
        FocusFixture::KOTLIN,
        crate::BuildProfile::Development,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_java_service_moves_traps_and_releases_focus_through_the_page() {
    if FocusFixture::skipped(super::java_tests::java_web_ready(), "TeaVM") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Java,
        "client/services/FocusService.java",
        FocusFixture::JAVA,
        crate::BuildProfile::Development,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_dart_service_moves_traps_and_releases_focus_through_the_page() {
    if FocusFixture::skipped(FocusFixture::answers("dart", "--version"), "Dart") {
        return;
    }
    let report = FocusFixture::run_dart().await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_ruby_service_moves_traps_and_releases_focus_through_the_page() {
    let unavailable = crate::tac_ruby::RubyRuntimeBuild::unavailable_reason();
    let ready = unavailable.is_none() && crate::node_driver::NodeDriver::available();
    if FocusFixture::skipped(ready, "Ruby WebAssembly") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Ruby,
        "client/services/FocusService.rb",
        FocusFixture::RUBY,
        crate::BuildProfile::Release,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_lua_service_moves_traps_and_releases_focus_through_the_page() {
    let unavailable = crate::tac_lua::LuaRuntimeBuild::unavailable_reason();
    let ready = unavailable.is_none() && crate::node_driver::NodeDriver::available();
    if FocusFixture::skipped(ready, "Lua WebAssembly") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Lua,
        "client/services/FocusService.lua",
        FocusFixture::LUA,
        crate::BuildProfile::Release,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}

#[tokio::test]
async fn a_cpp_service_moves_traps_and_releases_focus_through_the_page() {
    if FocusFixture::skipped(FocusFixture::answers("em++", "--version"), "Emscripten") {
        return;
    }
    let report = FocusFixture::run_service(
        super::WebLanguage::Cpp,
        "client/services/FocusService.hpp",
        FocusFixture::CPP,
        crate::BuildProfile::Release,
    )
    .await;
    assert!(report.contains("focus-service-passed"), "{report}");
}
