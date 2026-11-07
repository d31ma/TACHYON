use super::script_named_needs_tests::ScriptNamedNeeds;
use crate::one_test_per_case_tests::one_test_per_case;
use serde_json::{Value, json};

struct ScriptReach;

impl ScriptReach {
    const LANGUAGES: [&'static str; 2] = ["js", "ts"];

    const PAGE: [(&'static str, &'static str); 7] = [
        (
            "client/pages/tac.html",
            "<ticks-stream /><chat-room /><inventory-feed /><products-list />",
        ),
        (
            "client/models/Tick.ts",
            include_str!("../../../../website/content/features/realtime/client/models/Tick.ts"),
        ),
        (
            "client/components/ticks/stream/tac.html",
            include_str!(
                "../../../../website/content/features/realtime/client/components/ticks/stream/tac.html"
            ),
        ),
        (
            "client/models/ChatMessage.ts",
            include_str!(
                "../../../../website/content/features/sockets/client/models/ChatMessage.ts"
            ),
        ),
        (
            "client/components/chat/room/tac.html",
            include_str!(
                "../../../../website/content/features/sockets/client/components/chat/room/tac.html"
            ),
        ),
        (
            "client/components/inventory/feed/tac.html",
            include_str!(
                "../../../../website/content/docs/storage/topics/client/components/inventory/feed/tac.html"
            ),
        ),
        (
            "client/components/products/list/tac.html",
            "<p>{products.load}</p><button on:click=\"products.add('rope')\">add</button>",
        ),
    ];

    const TYPESCRIPT: [(&'static str, &'static str); 8] = [
        (
            "client/remotes/TicksClient.ts",
            include_str!(
                "../../../../website/content/features/realtime/client/remotes/TicksClient.ts"
            ),
        ),
        (
            "client/services/StreamService.ts",
            include_str!(
                "../../../../website/content/features/realtime/client/services/StreamService.ts"
            ),
        ),
        (
            "client/remotes/ChatClient.ts",
            include_str!(
                "../../../../website/content/features/sockets/client/remotes/ChatClient.ts"
            ),
        ),
        (
            "client/services/ChatService.ts",
            include_str!(
                "../../../../website/content/features/sockets/client/services/ChatService.ts"
            ),
        ),
        (
            "client/models/InventoryRecord.ts",
            include_str!(
                "../../../../website/content/docs/storage/topics/client/models/InventoryRecord.ts"
            ),
        ),
        (
            "client/services/InventoryService.ts",
            include_str!(
                "../../../../website/content/docs/storage/topics/client/services/InventoryService.ts"
            ),
        ),
        (
            "client/remotes/ProductsClient.ts",
            "@HTTPClient(\"/api\")\nexport default class ProductsClient {\n  \
             @GetRequest(\"/products\")\n  async all(@RequestParam(\"mode\") mode: string): Promise<BehaviorJsonValue | void> {}\n\n  \
             @PostRequest(\"/products\")\n  async add(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue | void> {}\n}\n",
        ),
        (
            "client/services/ProductsService.ts",
            "@Service\nexport default class ProductsService {\n  @Needs\n  readonly productsClient!: ProductsClient;\n\n  \
             async load(mode: string): Promise<BehaviorJsonValue | void> {\n    return this.productsClient.all(mode);\n  }\n\n  \
             async add(name: string): Promise<BehaviorJsonValue | void> {\n    return this.productsClient.add({ name });\n  }\n}\n",
        ),
    ];

    const JAVASCRIPT: [(&'static str, &'static str); 8] = [
        (
            "client/remotes/TicksClient.js",
            include_str!(
                "../../../../website/content/features/realtime/client/remotes/TicksClient.js"
            ),
        ),
        (
            "client/services/StreamService.js",
            include_str!(
                "../../../../website/content/features/realtime/client/services/StreamService.js"
            ),
        ),
        (
            "client/remotes/ChatClient.js",
            include_str!(
                "../../../../website/content/features/sockets/client/remotes/ChatClient.js"
            ),
        ),
        (
            "client/services/ChatService.js",
            include_str!(
                "../../../../website/content/features/sockets/client/services/ChatService.js"
            ),
        ),
        (
            "client/models/InventoryRecord.js",
            include_str!(
                "../../../../website/content/docs/storage/topics/client/models/InventoryRecord.js"
            ),
        ),
        (
            "client/services/InventoryService.js",
            include_str!(
                "../../../../website/content/docs/storage/topics/client/services/InventoryService.js"
            ),
        ),
        (
            "client/remotes/ProductsClient.js",
            "@HTTPClient(\"/api\")\nexport default class ProductsClient {\n  \
             /**\n   * @param {string} mode\n   * @returns {Promise<BehaviorJsonValue | void>}\n   */\n  \
             @GetRequest(\"/products\")\n  async all(@RequestParam(\"mode\") mode) {}\n\n  \
             /**\n   * @param {BehaviorJsonValue} product\n   * @returns {Promise<BehaviorJsonValue | void>}\n   */\n  \
             @PostRequest(\"/products\")\n  async add(@RequestBody product) {}\n}\n",
        ),
        (
            "client/services/ProductsService.js",
            "@Service\nexport default class ProductsService {\n  /** @type {ProductsClient} */\n  @Needs\n  productsClient;\n\n  \
             /**\n   * @param {string} mode\n   * @returns {Promise<BehaviorJsonValue | void>}\n   */\n  \
             async load(mode) {\n    return this.productsClient.all(mode);\n  }\n\n  \
             /**\n   * @param {string} name\n   * @returns {Promise<BehaviorJsonValue | void>}\n   */\n  \
             async add(name) {\n    return this.productsClient.add({ name });\n  }\n}\n",
        ),
    ];

    const DRIVER: &'static str = "globalThis.location = { href: 'http://127.0.0.1:4000/page', origin: 'http://127.0.0.1:4000' }\n\
         const opened = [], requested = [], posted = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); opened.push(this) } addEventListener() {} close() {} }\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; opened.push(this) } send(text) { this.sent.push(text) } close() {} }\n\
         globalThis.fetch = async (url, init) => {\n\
         \x20 if (init.method === 'POST') posted.push({ url: String(url), body: String(init.body) })\n\
         \x20 else requested.push({ url: String(url), method: init.method })\n\
         \x20 return new Response(JSON.stringify({ items: ['anvil'] }), { headers: { 'content-type': 'application/json' } })\n\
         }\n\
         const { default: ApplicationEvents } = await import('./application-events.mjs')\n\
         const bus = new ApplicationEvents({ changed: () => {}, failed: (error) => { throw error } })\n\
         const signal = new AbortController().signal\n\
         const open = async (name) => {\n\
         \x20 const module = await import(`./.tachyon/services/${name}.js`)\n\
         \x20 const root = await module.__tachyon_create_owner__({}, signal, undefined)\n\
         \x20 const graph = module.__tachyon_event_owners__(root)\n\
         \x20 for (const instance of graph) bus.attach(instance, module.__tachyon_event_plan__.types.find((type) => type.name === instance.constructor.name) || {}, { applicationEventPublisher: null })\n\
         \x20 for (const instance of graph) await bus.initialize(instance)\n\
         \x20 return root\n\
         }\n\
         const products = await open('ProductsService'), stream = await open('StreamService'), chat = await open('ChatService'), inventory = await open('InventoryService')\n\
         const found = (path) => opened.find((source) => source.url.split('?')[0].endsWith(path))\n\
         const ticks = found('/ticks'), socket = found('/chat'), topic = found('/.tachyon/topics/inventory-record')\n\
         const loaded = await products.load('short')\n\
         const added = await products.add('rope')\n\
         ticks.onmessage({ data: JSON.stringify({ sku: 'anvil', level: 3 }), lastEventId: '' })\n\
         socket.readyState = 1\n\
         socket.onopen?.()\n\
         await chat.sayHello()\n\
         socket.onmessage({ data: JSON.stringify({ text: 'welcome' }) })\n\
         topic.onmessage({ data: JSON.stringify({ sku: 'rope', level: 7 }), lastEventId: '1' })\n\
         console.log(JSON.stringify({ requested, posted, loaded, added, urls: opened.map((source) => source.url).sort(), ticks: stream.ticks, sent: socket.sent, messages: chat.messages, records: inventory.records }))\n\
         process.exit(0)\n";

    fn driver() -> String {
        let events = concat!(
            include_str!("../template/application-events.js"),
            "\nexport default TacApplicationEvents\n"
        );
        let quoted = serde_json::to_string(events).unwrap_or_else(|failure| panic!("{failure}"));
        format!(
            "import {{ writeFile }} from 'node:fs/promises'\nawait writeFile('./application-events.mjs', {quoted})\n{}",
            Self::DRIVER
        )
    }

    fn files(extension: &str) -> &'static [(&'static str, &'static str)] {
        match extension {
            "js" => &Self::JAVASCRIPT,
            "ts" => &Self::TYPESCRIPT,
            _ => panic!("no script reach project uses .{extension}"),
        }
    }

    async fn report(extension: &str) -> Value {
        let project = Self::PAGE
            .iter()
            .chain(Self::files(extension))
            .copied()
            .collect::<Vec<_>>();
        ScriptNamedNeeds::run(&project, &Self::driver()).await
    }

    fn assert_opened(report: &Value) {
        assert_eq!(
            report["urls"],
            json!([
                "/.tachyon/topics/inventory-record",
                "/ticks",
                "ws://127.0.0.1:4000/chat"
            ]),
            "{report}"
        );
    }
}

async fn request_param_reaches_the_sent_url_in(extension: &str) {
    let report = ScriptReach::report(extension).await;
    assert_eq!(
        report["requested"],
        json!([{"url": "http://127.0.0.1:4000/api/products?mode=short", "method": "GET"}]),
        "{report}"
    );
    assert_eq!(report["loaded"], json!({"items": ["anvil"]}), "{report}");
}

async fn post_request_sends_its_body_in(extension: &str) {
    let report = ScriptReach::report(extension).await;
    assert_eq!(
        report["posted"],
        json!([{"url": "http://127.0.0.1:4000/api/products", "body": "{\"name\":\"rope\"}"}]),
        "{report}"
    );
    assert_eq!(report["added"], json!({"items": ["anvil"]}), "{report}");
}

async fn stream_request_delivers_its_events_to_a_listener_in(extension: &str) {
    let report = ScriptReach::report(extension).await;
    ScriptReach::assert_opened(&report);
    assert_eq!(
        report["ticks"],
        json!([{"sku": "anvil", "level": 3}]),
        "{report}"
    );
}

async fn socket_request_sends_and_receives_messages_in(extension: &str) {
    let report = ScriptReach::report(extension).await;
    ScriptReach::assert_opened(&report);
    assert_eq!(
        report["sent"],
        json!(["{\"text\":\"Hello from a tab\"}"]),
        "{report}"
    );
    assert_eq!(report["messages"], json!([{"text": "welcome"}]), "{report}");
}

async fn topic_model_is_heard_from_its_topic_stream_in(extension: &str) {
    let report = ScriptReach::report(extension).await;
    ScriptReach::assert_opened(&report);
    assert_eq!(
        report["records"],
        json!([{"sku": "rope", "level": 7}]),
        "{report}"
    );
}

one_test_per_case!(
    a_script_request_param_reaches_the_sent_url,
    async request_param_reaches_the_sent_url_in,
    all ScriptReach::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

one_test_per_case!(
    a_script_post_request_sends_its_body,
    async post_request_sends_its_body_in,
    all ScriptReach::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

one_test_per_case!(
    a_script_stream_request_delivers_its_events_to_a_listener,
    async stream_request_delivers_its_events_to_a_listener_in,
    all ScriptReach::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

one_test_per_case!(
    a_script_socket_request_sends_and_receives_messages,
    async socket_request_sends_and_receives_messages_in,
    all ScriptReach::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);

one_test_per_case!(
    a_script_topic_model_is_heard_from_its_topic_stream,
    async topic_model_is_heard_from_its_topic_stream_in,
    all ScriptReach::LANGUAGES,
    [javascript => "js", typescript => "ts"]
);
