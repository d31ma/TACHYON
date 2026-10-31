use super::script_named_needs_tests::ScriptNamedNeeds;
use crate::one_test_per_case_tests::one_test_per_case;
use serde_json::{Value, json};

struct ScriptPageServices;

impl ScriptPageServices {
    const LANGUAGES: [&'static str; 2] = ["js", "ts"];

    const EDITOR_PAGE: [(&'static str, &'static str); 2] = [
        (
            "client/pages/tac.html",
            include_str!("../../../../website/content/features/persisted/client/pages/tac.html"),
        ),
        (
            "client/components/draft/editor/tac.html",
            include_str!(
                "../../../../website/content/features/persisted/client/components/draft/editor/tac.html"
            ),
        ),
    ];

    const JAVASCRIPT_EDITOR: [(&'static str, &'static str); 1] = [(
        "client/services/EditorService.js",
        include_str!(
            "../../../../website/content/features/persisted/client/services/EditorService.js"
        ),
    )];

    const TYPESCRIPT_EDITOR: [(&'static str, &'static str); 1] = [(
        "client/services/EditorService.ts",
        include_str!(
            "../../../../website/content/features/persisted/client/services/EditorService.ts"
        ),
    )];

    const TALLY_PAGE: [(&'static str, &'static str); 2] = [
        ("client/pages/tac.html", "<tally-card /><tally-card />"),
        (
            "client/components/tally/card/tac.html",
            "<p><button on:click=\"tally.add()\">+</button>{tally.count} {ledger.count}<button on:click=\"ledger.add()\">+</button></p>",
        ),
    ];

    const JAVASCRIPT_TALLY: [(&'static str, &'static str); 2] = [
        (
            "client/services/TallyService.js",
            "@Service\n@Lifetime({\"value\":\"prototype\"})\nexport default class TallyService {\n  count = 0\n\n  \
             add() {\n    this.count += 1\n    return this.count\n  }\n}\n",
        ),
        (
            "client/services/LedgerService.js",
            "@Service\nexport default class LedgerService {\n  count = 0\n\n  \
             add() {\n    this.count += 1\n    return this.count\n  }\n}\n",
        ),
    ];

    const TYPESCRIPT_TALLY: [(&'static str, &'static str); 2] = [
        (
            "client/services/TallyService.ts",
            "@Service\n@Lifetime({\"value\":\"prototype\"})\nexport default class TallyService {\n  count: number = 0\n\n  \
             add(): number {\n    this.count += 1\n    return this.count\n  }\n}\n",
        ),
        (
            "client/services/LedgerService.ts",
            "@Service\nexport default class LedgerService {\n  count: number = 0\n\n  \
             add(): number {\n    this.count += 1\n    return this.count\n  }\n}\n",
        ),
    ];

    const PAGE_RUNTIME: &'static str = "import { readFile } from 'node:fs/promises'\n\
         const client = await readFile('./.tachyon/tac-client.js', 'utf8')\n\
         const between = (from, to) => client.slice(client.indexOf(from), client.indexOf(to))\n\
         const runtime = `const attachRuntime = (owner, scope) => bindPersistentFields(owner, scope)\\n${between('const webStorage = ', 'const importAsset = ')}\\n${between('const createOwner = ', 'const snapshotState = ')}\\nreturn componentOwner`\n\
         const index = await readFile('./index.html', 'utf8')\n\
         const plan = JSON.parse(index.match(/<script id=\"tachyon-view\"[^>]*>([\\s\\S]*?)<\\/script>/)[1])\n\
         class MemoryStorage {\n\
         \x20 constructor() { this.items = new Map() }\n\
         \x20 getItem(key) { return this.items.has(key) ? this.items.get(key) : null }\n\
         \x20 setItem(key, value) { this.items.set(key, String(value)) }\n\
         \x20 removeItem(key) { this.items.delete(key) }\n\
         }\n\
         const local = new MemoryStorage()\n\
         const open = (session) => new Function('localStorage', 'sessionStorage', 'location', 'ClientView', 'importAsset', 'clientRequest', 'nativeHost', 'bindNativeInstance', 'drainCompanionQueue', 'disposeNativeInstance', 'Runtime', 'disposeRuntime', 'Rendering', 'Components', runtime)(\n\
         \x20 local, session, { pathname: '/' }, { plan }, (raw) => import(`.${raw}`), fetch, () => false, () => {}, () => {}, () => {},\n\
         \x20 { graphs: new WeakMap() }, () => {}, { instanceKeys: new Set() }, { instances: new Map(), records: new Map() })\n\
         const node = (name) => plan.nodes.find((entry) => entry.name === name)\n";

    const EDITOR_DRIVER: &'static str = "const tab = new MemoryStorage()\n\
         const page = (await open(tab)(node('draft-editor'), {}, 'body/0')).editor\n\
         const fresh = { draft: page.draft, theme: page.theme }\n\
         page.draft = 'chapter one'\n\
         page.theme = 'dark'\n\
         const reloaded = (await open(tab)(node('draft-editor'), {}, 'body/0')).editor\n\
         const other = (await open(new MemoryStorage())(node('draft-editor'), {}, 'body/0')).editor\n\
         console.log(JSON.stringify({ stored: plan.services.editor.stored, fresh, afterReload: { draft: reloaded.draft, theme: reloaded.theme }, newTab: { draft: other.draft, theme: other.theme }, saved: Object.fromEntries(local.items), session: Object.fromEntries(tab.items) }))\n";

    const TALLY_DRIVER: &'static str = "const page = open(new MemoryStorage())\n\
         const first = await page(node('tally-card'), {}, 'body/0')\n\
         const second = await page(node('tally-card'), {}, 'body/1')\n\
         first.tally.add()\n\
         first.tally.add()\n\
         first.ledger.add()\n\
         console.log(JSON.stringify({ scopes: { tally: plan.services.tally.scope, ledger: plan.services.ledger.scope }, tallies: [first.tally.count, second.tally.count], ledgers: [first.ledger.count, second.ledger.count], oneLedger: first.ledger === second.ledger }))\n";

    async fn report(
        page: &[(&'static str, &'static str)],
        services: &[(&'static str, &'static str)],
        driver: &str,
    ) -> Value {
        let project = page.iter().chain(services).copied().collect::<Vec<_>>();
        let driver = format!("{}{driver}", Self::PAGE_RUNTIME);
        ScriptNamedNeeds::run(&project, &driver).await
    }

    async fn editor(extension: &str) -> Value {
        let services: &[(&'static str, &'static str)] = match extension {
            "js" => &Self::JAVASCRIPT_EDITOR,
            "ts" => &Self::TYPESCRIPT_EDITOR,
            _ => panic!("no persisted editor uses .{extension}"),
        };
        Self::report(&Self::EDITOR_PAGE, services, Self::EDITOR_DRIVER).await
    }

    async fn tally(extension: &str) -> Value {
        let services: &[(&'static str, &'static str)] = match extension {
            "js" => &Self::JAVASCRIPT_TALLY,
            "ts" => &Self::TYPESCRIPT_TALLY,
            _ => panic!("no tally page uses .{extension}"),
        };
        Self::report(&Self::TALLY_PAGE, services, Self::TALLY_DRIVER).await
    }
}

async fn local_state_field_is_restored_after_a_reload_in(extension: &str) {
    let report = ScriptPageServices::editor(extension).await;
    assert_eq!(report["stored"]["theme"], "local", "{report}");
    assert_eq!(report["fresh"]["theme"], "light", "{report}");
    assert_eq!(report["afterReload"]["theme"], "dark", "{report}");
    assert_eq!(report["newTab"]["theme"], "dark", "{report}");
    assert_eq!(
        report["saved"],
        json!({"tac:service:editor:/:theme": "\"dark\""}),
        "{report}"
    );
}

async fn session_state_field_survives_a_reload_but_not_a_new_tab_in(extension: &str) {
    let report = ScriptPageServices::editor(extension).await;
    assert_eq!(report["stored"]["draft"], "session", "{report}");
    assert_eq!(report["fresh"]["draft"], "", "{report}");
    assert_eq!(report["afterReload"]["draft"], "chapter one", "{report}");
    assert_eq!(report["newTab"]["draft"], "", "{report}");
    assert_eq!(
        report["session"],
        json!({"tac:service:editor:/:draft": "\"chapter one\""}),
        "{report}"
    );
}

async fn prototype_service_gives_each_component_its_own_object_in(extension: &str) {
    let report = ScriptPageServices::tally(extension).await;
    assert_eq!(
        report["scopes"],
        json!({"tally": "prototype", "ledger": "singleton"}),
        "{report}"
    );
    assert_eq!(report["tallies"], json!([2, 0]), "{report}");
    assert_eq!(report["ledgers"], json!([1, 1]), "{report}");
    assert_eq!(report["oneLedger"], true, "{report}");
}

one_test_per_case!(
    a_script_local_state_field_is_restored_after_a_reload,
    async local_state_field_is_restored_after_a_reload_in,
    all ScriptPageServices::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

one_test_per_case!(
    a_script_session_state_field_survives_a_reload_but_not_a_new_tab,
    async session_state_field_survives_a_reload_but_not_a_new_tab_in,
    all ScriptPageServices::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

one_test_per_case!(
    a_script_prototype_service_gives_each_component_its_own_object,
    async prototype_service_gives_each_component_its_own_object_in,
    all ScriptPageServices::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);
