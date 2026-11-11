use super::backend_tests::PythonPage;
use super::python_browser_apis::PythonBrowserApis;
use super::python_provider_source::PythonProviderSource;
use crate::route_types::ModelTable;
use crate::web_api_clients::WebApiClients;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;

struct BrowserApiPage;

impl BrowserApiPage {
    const STAMP: [(&'static str, &'static str); 1] = [(
        "client/services/StampService.py",
        "from typing import Annotated\n\n\
         @service\n\
         class StampService:\n\
         \x20   clock: Annotated[Clock, needs]\n\
         \x20   secure_random: Annotated[SecureRandom, needs]\n\
         \x20   connectivity: Annotated[Connectivity, needs]\n\
         \x20   page_visibility: Annotated[PageVisibility, needs]\n\
         \x20   preferences: Annotated[Preferences, needs]\n\
         \x20   clipboard: Annotated[Clipboard, needs]\n\
         \x20   share: Annotated[Share, needs]\n\
         \x20   notifications: Annotated[Notifications, needs]\n\
         \x20   geolocation: Annotated[Geolocation, needs]\n\
         \x20   window_controls: Annotated[WindowControls, needs]\n\
         \x20   focus: Annotated[Focus, needs]\n\
         \x20   place: str = ''\n\n\
         \x20   def times(self) -> str:\n\
         \x20       return f'{self.clock.now_ms()} {self.clock.monotonic_ms()} {self.clock.time_zone()}'\n\n\
         \x20   def random(self) -> str:\n\
         \x20       return f'{list(self.secure_random.bytes(4))} {self.secure_random.uuid()}'\n\n\
         \x20   def status(self) -> str:\n\
         \x20       return f'{self.connectivity.online()} {self.page_visibility.visible()} {self.preferences.scheme()} {self.preferences.motion()} {self.preferences.language()}'\n\n\
         \x20   def consent(self) -> str:\n\
         \x20       self.clipboard.write_text('atlas')\n\
         \x20       self.clipboard.read_text()\n\
         \x20       self.share.open('Atlas', 'A map', 'https://example.com/atlas')\n\
         \x20       self.notifications.request_permission()\n\
         \x20       self.notifications.show('Atlas', 'Ready')\n\
         \x20       self.geolocation.current_position()\n\
         \x20       return self.notifications.permission()\n\n\
         \x20   def window(self) -> bool:\n\
         \x20       self.window_controls.minimize()\n\
         \x20       self.window_controls.maximize()\n\
         \x20       self.window_controls.fullscreen()\n\
         \x20       self.window_controls.close()\n\
         \x20       self.window_controls.resize(800, 600)\n\
         \x20       self.window_controls.set_title('Atlas')\n\
         \x20       return self.window_controls.granted('close')\n\n\
         \x20   def dialog(self) -> None:\n\
         \x20       self.focus.trap('dialog')\n\
         \x20       self.focus.move('name')\n\
         \x20       self.focus.release()\n\n\
         \x20   def refused(self) -> str:\n\
         \x20       try:\n\
         \x20           self.secure_random.bytes(65537)\n\
         \x20       except RuntimeError as error:\n\
         \x20           return str(error)\n\
         \x20       return ''\n\n\
         \x20   @on_event\n\
         \x20   def found(self, found: PositionFound) -> None:\n\
         \x20       self.place = f'{found.latitude},{found.longitude}'\n",
    )];

    const DRIVER: &'static str = "_requests = []\n\
         _answers = {'clock.now': 1000, 'clock.monotonic': 5.5, 'clock.timeZone': 'Europe/Paris',\n\
         \x20   'secureRandom.bytes': [1, 2, 3, 4], 'secureRandom.uuid': 'uuid-1', 'connectivity.online': False,\n\
         \x20   'pageVisibility.visible': True, 'preferences.scheme': 'dark', 'preferences.motion': 'reduce',\n\
         \x20   'preferences.language': 'fr', 'notifications.permission': 'granted', 'windowControls.granted': True}\n\
         def _web(text):\n\
         \x20   request = json.loads(text)\n\
         \x20   _requests.append(request)\n\
         \x20   if request['args'] == [65537]:\n\
         \x20       return json.dumps({'error': 'At most 65536 random bytes.'})\n\
         \x20   return json.dumps({'value': _answers.get(request['api'] + '.' + request['call'])})\n\
         _TacPythonRuntime.connect(_emitted.append, None, web=_web)\n\
         _call = lambda name: _ask('stamp', {'op': 'call', 'name': name, 'args': []})\n\
         _report = {'init': _ask('stamp', {'op': 'init', 'props': {}})}\n\
         _report.update({name: _call(name) for name in ['times', 'random', 'status', 'consent', 'window', 'dialog', 'refused']})\n\
         _report['heard'] = _ask('stamp', {'op': 'event', 'name': 'found', 'value': {'latitude': 1.5, 'longitude': 2.5, 'accuracy': 3.0, 'timestamp': 4.0}})\n\
         _report['place'] = _ask('stamp', {'op': 'get', 'name': 'place'})\n\
         _report['requests'] = [[item['api'], item['call'], item['args']] for item in _requests]\n";

    const ALERTS: [(&'static str, &'static str); 1] = [(
        "client/services/StampService.py",
        "from typing import Annotated\n\n\
         @service\n\
         class StampService:\n\
         \x20   push: Annotated[Push, needs]\n\
         \x20   endpoint: str = ''\n\n\
         \x20   def turn_on(self) -> bool:\n\
         \x20       self.push.subscribe('server-key')\n\
         \x20       self.push.subscription()\n\
         \x20       self.push.unsubscribe()\n\
         \x20       return self.push.supported()\n\n\
         \x20   @on_event\n\
         \x20   def subscribed(self, subscribed: PushSubscribed) -> None:\n\
         \x20       self.endpoint = subscribed.endpoint\n\n\
         \x20   @on_event\n\
         \x20   def received(self, pushed: PushReceived) -> None:\n\
         \x20       self.endpoint = f'{pushed.tag} {pushed.path} {pushed.data}'\n",
    )];

    const ALERTS_DRIVER: &'static str = "_requests = []\n\
         def _web(text):\n\
         \x20   request = json.loads(text)\n\
         \x20   _requests.append(request)\n\
         \x20   return json.dumps({'value': request['call'] == 'supported'})\n\
         _TacPythonRuntime.connect(_emitted.append, None, web=_web)\n\
         _report = {'init': _ask('stamp', {'op': 'init', 'props': {}})}\n\
         _report['turn_on'] = _ask('stamp', {'op': 'call', 'name': 'turnOn', 'args': []})\n\
         _report['heard'] = _ask('stamp', {'op': 'event', 'name': 'subscribed', 'value': {'endpoint': 'https://push.example/1', 'p256dh': 'key', 'auth': 'secret', 'expirationTime': 0.0}})\n\
         _report['endpoint'] = _ask('stamp', {'op': 'get', 'name': 'endpoint'})\n\
         _report['pushed'] = _ask('stamp', {'op': 'event', 'name': 'received', 'value': {'tag': 'mail-7', 'path': '/mail/7', 'data': '{\"thread\":7}'}})\n\
         _report['received'] = _ask('stamp', {'op': 'get', 'name': 'endpoint'})\n\
         _report['requests'] = [[item['api'], item['call'], item['args']] for item in _requests]\n";

    const SPACE: [(&'static str, &'static str); 1] = [(
        "client/services/StampService.py",
        "from typing import Annotated\n\n\
         @service\n\
         class StampService:\n\
         \x20   storage_space: Annotated[StorageSpace, needs]\n\
         \x20   usage: float = 0.0\n\
         \x20   kept: bool = False\n\n\
         \x20   def measure(self) -> None:\n\
         \x20       self.storage_space.estimate()\n\
         \x20       self.storage_space.persist()\n\n\
         \x20   @on_event\n\
         \x20   def estimated(self, estimated: StorageEstimated) -> None:\n\
         \x20       self.usage = estimated.usage\n\n\
         \x20   @on_event\n\
         \x20   def persisted(self, persisted: StoragePersisted) -> None:\n\
         \x20       self.kept = persisted.persisted\n",
    )];

    const SPACE_DRIVER: &'static str = "_requests = []\n\
         def _web(text):\n\
         \x20   _requests.append(json.loads(text))\n\
         \x20   return json.dumps({'value': None})\n\
         _TacPythonRuntime.connect(_emitted.append, None, web=_web)\n\
         _report = {'init': _ask('stamp', {'op': 'init', 'props': {}})}\n\
         _report['measure'] = _ask('stamp', {'op': 'call', 'name': 'measure', 'args': []})\n\
         _report['estimated'] = _ask('stamp', {'op': 'event', 'name': 'estimated', 'value': {'usage': 2048.0, 'quota': 4096.0, 'persisted': False}})\n\
         _report['persisted'] = _ask('stamp', {'op': 'event', 'name': 'persisted', 'value': {'persisted': True}})\n\
         _report['usage'] = _ask('stamp', {'op': 'get', 'name': 'usage'})\n\
         _report['kept'] = _ask('stamp', {'op': 'get', 'name': 'kept'})\n\
         _report['requests'] = [[item['api'], item['call'], item['args']] for item in _requests]\n";

    const ATTACH: [(&'static str, &'static str); 1] = [(
        "client/services/AttachService.py",
        "from typing import Annotated\n\n\
         @service\n\
         class AttachService:\n\
         \x20   files: Annotated[Files, needs]\n\
         \x20   name: str = ''\n\n\
         \x20   def choose(self) -> None:\n\
         \x20       self.files.pick('.txt', True)\n\n\
         \x20   def store(self) -> str:\n\
         \x20       self.files.read('file-1', 0, 3)\n\
         \x20       upload = self.files.upload('file-1', 'PUT', '/uploads', 'content-type: text/plain')\n\
         \x20       self.files.cancel(upload)\n\
         \x20       self.files.release('file-1')\n\
         \x20       return upload\n\n\
         \x20   @on_event\n\
         \x20   def picked(self, picked: FilePicked) -> None:\n\
         \x20       self.name = f'{picked.name} {picked.media} {picked.sha256}'\n",
    )];

    const FILES_DRIVER: &'static str = "_requests = []\n\
         def _web(text):\n\
         \x20   request = json.loads(text)\n\
         \x20   _requests.append(request)\n\
         \x20   return json.dumps({'value': 'upload-1' if request['call'] == 'upload' else None})\n\
         _TacPythonRuntime.connect(_emitted.append, None, web=_web)\n\
         _call = lambda name: _ask('attach', {'op': 'call', 'name': name, 'args': []})\n\
         _report = {'init': _ask('attach', {'op': 'init', 'props': {}})}\n\
         _report.update({name: _call(name) for name in ['choose', 'store']})\n\
         _report['heard'] = _ask('attach', {'op': 'event', 'name': 'picked', 'value': {'id': 'file-1', 'name': 'a.txt', 'size': 3, 'media': 'text/plain', 'sha256': 'ab12', 'index': 0, 'count': 1}})\n\
         _report['name'] = _ask('attach', {'op': 'get', 'name': 'name'})\n\
         _report['requests'] = [[item['api'], item['call'], item['args']] for item in _requests]\n";

    fn provider(source: &str) -> PythonProviderSource {
        PythonProviderSource::read(Path::new("client/services/StampService.py"), source)
            .unwrap_or_else(|error| panic!("provider: {error:?}"))
    }

    fn models() -> ModelTable {
        let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
        ModelTable::read_realm(root.path(), "client", "py")
            .unwrap_or_else(|error| panic!("models: {error}"))
    }
}

#[test]
fn every_shared_browser_api_has_a_python_client() {
    for name in WebApiClients::TYPE_NAMES {
        assert!(PythonBrowserApis::is_client(name), "{name}");
    }
    let source = PythonBrowserApis::source();
    for name in WebApiClients::TYPE_NAMES {
        assert!(source.contains(&format!("class {name}:")), "{name}");
    }
}

#[test]
fn python_needs_browser_api_fields_are_given_by_the_page() {
    let provider = BrowserApiPage::provider(
        "from typing import Annotated\n\n@service\nclass StampService:\n    clock: Annotated[Clock, needs]\n    label: str = ''\n",
    );
    let description = provider
        .describe(&BTreeSet::new(), &BrowserApiPage::models())
        .unwrap_or_else(|error| panic!("browser API binding: {error:?}"));
    assert_eq!(
        description["constructor"],
        json!([{"browserApi": "Clock", "name": "clock"}])
    );
    assert!(
        description["fields"].get("clock").is_none(),
        "{description}"
    );
    assert_eq!(description["fields"]["label"]["shape"], "text");
}

#[test]
fn a_python_needs_all_browser_api_field_is_given_by_the_page() {
    let provider = BrowserApiPage::provider(
        "@service\n@needs_all\nclass StampService:\n    random: SecureRandom\n    title: str\n",
    );
    let description = provider
        .describe(&BTreeSet::new(), &BrowserApiPage::models())
        .unwrap_or_else(|error| panic!("needs all binding: {error:?}"));
    assert_eq!(
        description["constructor"][0],
        json!({"browserApi": "SecureRandom", "name": "random"})
    );
    assert_eq!(description["constructor"][1]["shape"], "text");
}

#[test]
fn a_python_browser_api_field_without_needs_says_how_to_mark_it() {
    let provider = BrowserApiPage::provider(
        "@service\nclass StampService:\n    clock: Clock\n    label: str = ''\n",
    );
    let failure = provider
        .describe(&BTreeSet::new(), &BrowserApiPage::models())
        .err()
        .unwrap_or_else(|| panic!("a plain field is not given by the page"));
    assert_eq!(failure.diagnostics()[0].code.number(), 1605, "{failure}");
    assert!(
        failure.to_string().contains("Annotated[Clock, needs]"),
        "{failure}"
    );
}

#[test]
fn a_needs_all_browser_api_field_is_injected_instead_of_a_page_property() {
    let root = tempfile::tempdir().unwrap_or_else(|error| panic!("project: {error}"));
    let path = root.path().join("client/services/StampService.py");
    std::fs::create_dir_all(path.parent().unwrap_or(root.path()))
        .unwrap_or_else(|error| panic!("folder: {error}"));
    std::fs::write(
        &path,
        "@service\n@needs_all\nclass StampService:\n    clock: Clock\n    title: str\n",
    )
    .unwrap_or_else(|error| panic!("source: {error}"));
    let registry = crate::template::ServiceRegistry::discover(root.path())
        .unwrap_or_else(|error| panic!("registry: {error:?}"));
    let service = registry
        .get("stamp")
        .unwrap_or_else(|| panic!("stamp service"));
    assert_eq!(service.required(), ["title"]);
    assert_eq!(service.injected(), ["clock"]);
}

#[test]
fn python_services_reach_every_browser_api_through_the_page() {
    if !PythonPage::has_python() {
        return;
    }
    let report = PythonPage::run(&BrowserApiPage::STAMP, &["stamp"], BrowserApiPage::DRIVER);
    assert_eq!(
        report["init"]["value"]["fields"],
        json!(["place"]),
        "{report}"
    );
    assert_eq!(
        report["times"],
        json!({"value": "1000.0 5.5 Europe/Paris"}),
        "{report}"
    );
    assert_eq!(report["random"], json!({"value": "[1, 2, 3, 4] uuid-1"}));
    assert_eq!(
        report["status"],
        json!({"value": "False True dark reduce fr"})
    );
    assert_eq!(report["consent"], json!({"value": "granted"}), "{report}");
    assert_eq!(report["window"], json!({"value": true}), "{report}");
    assert_eq!(report["dialog"], json!({"value": null}), "{report}");
    assert_eq!(
        report["refused"],
        json!({"value": "At most 65536 random bytes."})
    );
    assert_eq!(report["heard"], json!({"value": null}), "{report}");
    assert_eq!(report["place"], json!({"value": "1.5,2.5"}), "{report}");
    assert_eq!(
        report["requests"],
        json!([
            ["clock", "now", []],
            ["clock", "monotonic", []],
            ["clock", "timeZone", []],
            ["secureRandom", "bytes", [4]],
            ["secureRandom", "uuid", []],
            ["connectivity", "online", []],
            ["pageVisibility", "visible", []],
            ["preferences", "scheme", []],
            ["preferences", "motion", []],
            ["preferences", "language", []],
            ["clipboard", "writeText", ["atlas"]],
            ["clipboard", "readText", []],
            [
                "share",
                "open",
                ["Atlas", "A map", "https://example.com/atlas"]
            ],
            ["notifications", "requestPermission", []],
            ["notifications", "show", ["Atlas", "Ready"]],
            ["geolocation", "currentPosition", []],
            ["notifications", "permission", []],
            ["windowControls", "minimize", []],
            ["windowControls", "maximize", []],
            ["windowControls", "fullscreen", []],
            ["windowControls", "close", []],
            ["windowControls", "resize", [800, 600]],
            ["windowControls", "setTitle", ["Atlas"]],
            ["windowControls", "granted", ["close"]],
            ["focus", "trap", ["dialog"]],
            ["focus", "move", ["name"]],
            ["focus", "release", []],
            ["secureRandom", "bytes", [65537]],
        ]),
        "{report}"
    );
}

#[test]
fn a_python_web_service_subscribes_to_web_push() {
    if !PythonPage::has_python() {
        return;
    }
    let report = PythonPage::run(
        &BrowserApiPage::ALERTS,
        &["stamp"],
        BrowserApiPage::ALERTS_DRIVER,
    );
    assert_eq!(report["turn_on"], json!({"value": true}), "{report}");
    assert_eq!(report["heard"], json!({"value": null}), "{report}");
    assert_eq!(
        report["endpoint"],
        json!({"value": "https://push.example/1"}),
        "{report}"
    );
    assert_eq!(report["pushed"], json!({"value": null}), "{report}");
    assert_eq!(
        report["received"],
        json!({"value": "mail-7 /mail/7 {\"thread\":7}"}),
        "{report}"
    );
    assert_eq!(
        report["requests"],
        json!([
            ["push", "subscribe", ["server-key"]],
            ["push", "subscription", []],
            ["push", "unsubscribe", []],
            ["push", "supported", []],
        ]),
        "{report}"
    );
}

#[test]
fn a_python_web_service_estimates_and_persists_storage() {
    if !PythonPage::has_python() {
        return;
    }
    let report = PythonPage::run(
        &BrowserApiPage::SPACE,
        &["stamp"],
        BrowserApiPage::SPACE_DRIVER,
    );
    assert_eq!(report["measure"], json!({"value": null}), "{report}");
    assert_eq!(report["estimated"], json!({"value": null}), "{report}");
    assert_eq!(report["persisted"], json!({"value": null}), "{report}");
    assert_eq!(report["usage"], json!({"value": 2048.0}), "{report}");
    assert_eq!(report["kept"], json!({"value": true}), "{report}");
    assert_eq!(
        report["requests"],
        json!([
            ["storageSpace", "estimate", []],
            ["storageSpace", "persist", []],
        ]),
        "{report}"
    );
}

#[test]
fn the_python_page_engine_gives_the_runtime_the_web_host() {
    let engine = include_str!("python_host.js");
    assert!(engine.contains("web: (text) => webHost(text)"));
    assert!(engine.contains("hosts?.socket, hosts?.web)"));
}

#[test]
fn python_services_pick_read_and_upload_files_through_the_page() {
    if !PythonPage::has_python() {
        return;
    }
    let report = PythonPage::run(
        &BrowserApiPage::ATTACH,
        &["attach"],
        BrowserApiPage::FILES_DRIVER,
    );
    assert_eq!(report["store"], json!({"value": "upload-1"}), "{report}");
    assert_eq!(
        report["name"],
        json!({"value": "a.txt text/plain ab12"}),
        "{report}"
    );
    assert_eq!(
        report["requests"],
        json!([
            ["files", "pick", [".txt", true]],
            ["files", "read", ["file-1", 0, 3]],
            [
                "files",
                "upload",
                ["file-1", "PUT", "/uploads", "content-type: text/plain"]
            ],
            ["files", "cancel", ["upload-1"]],
            ["files", "release", ["file-1"]],
        ]),
        "{report}"
    );
}
