#![allow(clippy::expect_used)]

use super::native_menu_action_check::NativeMenuActionCheck;
use super::native_menu_config::NativeMenuConfig;
use super::registry::NativeCompanionInput;
use crate::project::NativeCompanion;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

pub(super) struct MenuActionFixture;

impl MenuActionFixture {
    const APPLICATION: &str = r#"
use crate::models::SectionChosen;

#[Service]
#[Lifetime({"value":"application"})]
struct SectionMenuService {
    page_events: CurrentPageEvents,
}

impl SectionMenuService {
    fn new(page_events: CurrentPageEvents) -> Self {
        Probe::record("created");
        Self { page_events }
    }

    #[OnStop]
    fn stopped(&self) {
        Probe::record("stopped");
    }

    fn show_running(&self) {
        let delivered = self.page_events.publish_event(SectionChosen { section: String::from("running") });
        Probe::record(if delivered { "forwarded" } else { "refused" });
    }

    fn refresh(&self) {
        Probe::record("refreshed");
    }
}
"#;
    const PAGE: &str = r#"
use crate::models::SectionChosen;

#[Service]
#[derive(Default)]
struct SectionService {
    section: String,
    visits: i64,
}

impl SectionService {
    fn visit(&mut self) {
        self.visits += 1;
    }

    #[OnEvent]
    fn chosen(&mut self, chosen: SectionChosen) {
        self.section = chosen.section;
        Probe::record("chosen");
        if !CurrentPageEvents::default().publish_event(SectionChosen { section: String::from("again") }) {
            Probe::record("nested.refused");
        }
    }
}
"#;
    const PROTOTYPE: &str = r#"
use crate::models::SectionChosen;

#[Service]
#[Lifetime({"value":"prototype"})]
#[derive(Default)]
struct TallyService {
    seen: i64,
}

impl TallyService {
    #[OnEvent]
    fn chosen(&mut self, _chosen: SectionChosen) {
        self.seen += 1;
        Probe::record("tallied");
    }
}
"#;
    pub(super) const PROBE: &str = r#"
struct Probe;
impl Probe {
    fn tape() -> &'static std::thread::LocalKey<core::cell::RefCell<Vec<String>>> {
        thread_local! { static VALUE: core::cell::RefCell<Vec<String>> = const { core::cell::RefCell::new(Vec::new()) }; }
        &VALUE
    }
    fn record(value: &str) { Self::tape().with(|tape| tape.borrow_mut().push(value.to_owned())); }
    fn count(value: &str) -> usize { Self::tape().with(|tape| tape.borrow().iter().filter(|event| event.as_str() == value).count()) }
}
#[unsafe(no_mangle)]
pub extern "C" fn tac_fixture_menu_count(event: *const core::ffi::c_char) -> i64 {
    let event = unsafe { core::ffi::CStr::from_ptr(event) }.to_str().unwrap_or_default();
    Probe::count(event) as i64
}
"#;
    const CALLER: &str = r###"
fn request(service: &str, owner: &str, operation: &str, name: &str) -> String {
    tac_handle(&format!(r##"{{"route":"/","service":"{service}","owner":"{owner}","op":"{operation}","name":"{name}","args":[]}}"##))
}
fn menu(id: &str) -> bool {
    let id = std::ffi::CString::new(id).unwrap();
    tac_native_menu_action(id.as_ptr())
}
fn ready(event: &str) -> bool {
    let event = std::ffi::CString::new(event).unwrap();
    tac_native_menu_ready(event.as_ptr())
}
fn main() {
    assert!(tac_native_application_start());
    assert!(!ready("SectionChosen"), "no page means no recipient");
    assert!(menu("show-running"));
    assert_eq!(Probe::count("refused"), 1, "the application service runs with no page open");
    assert!(menu("refresh"));
    assert_eq!(Probe::count("refreshed"), 1);
    let first = "00000000-0000-4000-8000-000000000001";
    assert!(!request("section", first, "init", "").contains("error"));
    let _ = request("section", first, "call", "visit");
    assert!(ready("SectionChosen"));
    assert!(!ready("OtherEvent"));
    assert!(menu("show-running"));
    assert_eq!(Probe::count("forwarded"), 1);
    assert_eq!(Probe::count("chosen"), 1);
    assert_eq!(Probe::count("nested.refused"), 1, "a listener cannot publish to the page again during delivery");
    assert_eq!(request("section", first, "get", "section"), "{\"value\":\"running\"}");
    assert_eq!(request("section", first, "get", "visits"), "{\"value\":1}", "page state stays");
    let second = "00000000-0000-4000-8000-000000000002";
    assert!(!request("section", second, "init", "").contains("error"));
    let tallies = ["00000000-0000-4000-8000-000000000003", "00000000-0000-4000-8000-000000000004"];
    for tally in tallies { assert!(!request("tally", tally, "init", "").contains("error")); }
    assert!(menu("show-running"));
    assert_eq!(Probe::count("chosen"), 2, "a page singleton bound twice receives one delivery");
    assert_eq!(Probe::count("tallied"), 2, "each prototype receives one delivery");
    let _ = tac_handle("{\"op\":\"host-dispose\"}");
    assert!(!ready("SectionChosen"), "navigation retires every recipient");
    assert!(menu("show-running"));
    assert_eq!(Probe::count("refused"), 2);
    assert!(!menu("unknown"));
    assert_eq!(Probe::count("created"), 1);
    tac_native_application_shutdown();
    tac_native_application_shutdown();
    assert_eq!(Probe::count("stopped"), 1, "Quit cleanup runs once");
    assert!(!menu("refresh"), "a retired application refuses menu actions");
    assert_eq!(Probe::count("refreshed"), 1);
    println!("native menu actions passed");
}
"###;

    pub(super) fn menus() -> NativeMenuConfig {
        serde_json::from_value(json!({
            "view": [
                {"id":"show-running","label":"What's Running","action":{"service":"SectionMenuService","method":"show_running","event":"SectionChosen"},"shortcut":{"key":"1","modifiers":["command"]}}
            ],
            "file": [
                {"id":"refresh","label":"Refresh","action":{"service":"SectionMenuService","method":"refresh"}}
            ]
        }))
        .expect("menu declarations")
    }

    fn write(path: &Path, source: &str) -> PathBuf {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
        std::fs::write(path, source).expect("fixture source");
        path.to_path_buf()
    }

    pub(super) fn inputs(root: &Path) -> Vec<NativeCompanionInput> {
        Self::write(
            &root.join("client/models/SectionChosen.rs"),
            "pub struct SectionChosen {\n    pub section: String,\n}\n",
        );
        [
            (
                "sectionMenu",
                "SectionMenuService",
                "menus/SectionMenuService.rs",
                Self::APPLICATION,
                true,
            ),
            (
                "section",
                "SectionService",
                "SectionService.rs",
                Self::PAGE,
                false,
            ),
            (
                "tally",
                "TallyService",
                "TallyService.rs",
                Self::PROTOTYPE,
                false,
            ),
        ]
        .into_iter()
        .map(
            |(service, class, file, text, application_provider)| NativeCompanionInput {
                snapshot_root: root.to_path_buf(),
                language: NativeCompanion::Rust,
                source: Self::write(&root.join("client/services").join(file), text),
                service: service.to_owned(),
                class: class.to_owned(),
                web_assembly: false,
                application_provider,
            },
        )
        .collect()
    }

    pub(super) fn stage(root: &Path) -> PathBuf {
        let source = super::rust::stage(&Self::inputs(root), root, "org.tachyon.menu-actions")
            .expect("staged Rust services")
            .expect("native Rust source");
        super::mac_os_menu_action_source::MacOsMenuActionSource::stage(
            Some(&source),
            &Self::menus(),
        )
        .expect("menu dispatch");
        let staged = std::fs::read_to_string(&source).expect("staged source");
        std::fs::write(&source, staged + Self::PROBE).expect("probe");
        source
    }

    fn refusal(menus: &Value) -> String {
        let root = tempfile::tempdir().expect("menu project");
        std::fs::create_dir_all(root.path().join("client")).expect("client directory");
        let manifest = json!({"name":"Menu Check","tac":{"application":{"menus":menus}}});
        std::fs::write(
            root.path().join("client/manifest.json"),
            manifest.to_string(),
        )
        .expect("manifest");
        super::config::NativeApplication::discover(root.path())
            .expect_err("invalid menu declaration")
            .to_string()
    }

    fn check(menus: &Value, inputs: &[NativeCompanionInput]) -> Result<(), crate::Failure> {
        let config: NativeMenuConfig = serde_json::from_value(menus.clone()).expect("typed menus");
        NativeMenuActionCheck::validate(&config, inputs)
    }
}

#[test]
fn native_menu_actions_need_exactly_one_destination_and_typed_names() {
    let action = json!({"service":"SectionMenuService","method":"show_running"});
    let both = MenuActionFixture::refusal(
        &json!({"file":[{"id":"x","label":"X","route":"/","action":action}]}),
    );
    assert!(both.contains("exactly one of route or action"), "{both}");
    let neither = MenuActionFixture::refusal(&json!({"file":[{"id":"x","label":"X"}]}));
    assert!(
        neither.contains("exactly one of route or action"),
        "{neither}"
    );
    for action in [
        json!({"service":"sectionMenuService","method":"show_running"}),
        json!({"service":"SectionMenuService","method":"ShowRunning"}),
        json!({"service":"SectionMenuService","method":"show-running"}),
        json!({"service":"SectionMenuService","method":"show_running","event":"section.chosen"}),
        json!({"service":"S".repeat(65),"method":"show_running"}),
    ] {
        let error =
            MenuActionFixture::refusal(&json!({"view":[{"id":"x","label":"X","action":action}]}));
        assert!(error.contains("service type name"), "{error}");
    }
    let close = MenuActionFixture::refusal(
        &json!({"file":[{"id":"x","label":"X","route":"/","shortcut":{"key":"w","modifiers":["command"]}}]}),
    );
    assert!(close.contains("File, Edit or Window"), "{close}");
}

#[test]
fn native_menu_actions_emit_typed_swift_items_and_a_generated_rust_dispatch() {
    let menus = MenuActionFixture::menus();
    let routes = super::mac_os_menu_source::MacOsMenuSource::declarations(&menus);
    assert_eq!(routes, "[]");
    let actions = super::mac_os_menu_source::MacOsMenuSource::actions(&menus);
    assert!(
        actions.contains(
            "MacOsMenuAction(id: \"refresh\", label: \"Refresh\", group: .file, event: \"\""
        ),
        "{actions}"
    );
    assert!(actions.contains("group: .view, event: \"SectionChosen\", shortcut: MacOsMenuShortcut(key: \"1\", modifiers: [.command])"), "{actions}");
    let dispatch = super::mac_os_menu_action_source::MacOsMenuActionSource::rust_dispatch(&menus);
    assert!(dispatch.contains("\"show-running\" => tac_with_application_SectionMenuService(|service| { let _ = service.show_running(); })"), "{dispatch}");
    assert!(
        dispatch.starts_with("pub(super) fn tac_menu_action(id: &str)"),
        "{dispatch}"
    );
    let source = super::mac_os_menu_action_source::MacOsMenuActionSource::swift_calls;
    assert!(source(&menus, true).contains("@_silgen_name(\"tac_native_menu_action\")"));
    assert!(!source(&menus, false).contains("@_silgen_name"));
    assert!(!source(&NativeMenuConfig::default(), true).contains("@_silgen_name"));
}

#[test]
fn native_menu_actions_require_an_application_rust_service_method_and_a_rust_model() {
    let root = tempfile::tempdir().expect("project");
    let inputs = MenuActionFixture::inputs(root.path());
    let menus = |action: Value| json!({"view":[{"id":"x","label":"X","action":action}]});
    MenuActionFixture::check(
        &menus(
            json!({"service":"SectionMenuService","method":"show_running","event":"SectionChosen"}),
        ),
        &inputs,
    )
    .expect("declared application action");
    for (action, expected) in [
        (
            json!({"service":"MissingService","method":"run"}),
            "application lifetime",
        ),
        (
            json!({"service":"SectionService","method":"visit"}),
            "application lifetime",
        ),
        (
            json!({"service":"SectionMenuService","method":"missing"}),
            "takes only &self",
        ),
        (
            json!({"service":"SectionMenuService","method":"new"}),
            "takes only &self",
        ),
        (
            json!({"service":"SectionMenuService","method":"refresh","event":"Missing"}),
            "not a Rust model",
        ),
        (
            json!({"service":"SectionMenuService","method":"refresh","event":"Section"}),
            "not a Rust model",
        ),
    ] {
        let failure =
            MenuActionFixture::check(&menus(action), &inputs).expect_err("invalid action");
        let text = failure.to_string();
        assert!(text.contains("TY1370"), "{text}");
        assert!(text.contains(expected), "{text}");
    }
    let swift = inputs
        .iter()
        .map(|input| NativeCompanionInput {
            language: NativeCompanion::Swift,
            ..input.clone()
        })
        .collect::<Vec<_>>();
    MenuActionFixture::check(
        &menus(json!({"service":"SectionMenuService","method":"refresh"})),
        &swift,
    )
    .expect_err("Swift providers have no application lifetime");
}

#[tokio::test]
async fn native_menu_actions_reach_live_page_listeners_once_and_refuse_without_a_recipient() {
    let root = tempfile::tempdir().expect("project");
    let source = MenuActionFixture::stage(root.path());
    super::native_page_graph_tests::NativePageGraphCheck::execute(
        &source,
        MenuActionFixture::CALLER,
    )
    .await;
}
