use super::script_named_needs_tests::ScriptNamedNeeds;
use serde_json::json;

struct TypeScriptReach;

impl TypeScriptReach {
    const FILES: [(&'static str, &'static str); 15] = [
        (
            "client/pages/tac.html",
            "<ticks-stream /><chat-room /><inventory-feed /><products-list />",
        ),
        (
            "client/models/Tick.ts",
            include_str!("../../../../website/content/features/realtime/client/models/Tick.ts"),
        ),
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
            "client/components/chat/room/tac.html",
            include_str!(
                "../../../../website/content/features/sockets/client/components/chat/room/tac.html"
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
            "client/components/inventory/feed/tac.html",
            include_str!(
                "../../../../website/content/docs/storage/topics/client/components/inventory/feed/tac.html"
            ),
        ),
        (
            "client/remotes/ProductsClient.ts",
            "@HTTPClient(\"/api\")\nexport default class ProductsClient {\n  \
             @GetRequest(\"/products\")\n  async all(@RequestParam(\"mode\") mode: string): Promise<BehaviorJsonValue | void> {}\n}\n",
        ),
        (
            "client/services/ProductsService.ts",
            "@Service\nexport default class ProductsService {\n  @Needs\n  readonly productsClient!: ProductsClient;\n\n  \
             async load(mode: string): Promise<BehaviorJsonValue | void> {\n    return this.productsClient.all(mode);\n  }\n}\n",
        ),
        (
            "client/components/products/list/tac.html",
            "<p>{products.load}</p>",
        ),
    ];

    const DRIVER: &'static str = "globalThis.location = { href: 'http://127.0.0.1:4000/page', origin: 'http://127.0.0.1:4000' }\n\
         const opened = [], requested = []\n\
         globalThis.EventSource = class { constructor(url) { this.url = String(url); opened.push(this) } addEventListener() {} close() {} }\n\
         globalThis.WebSocket = class { constructor(url) { this.url = String(url); this.readyState = 0; this.sent = []; opened.push(this) } send(text) { this.sent.push(text) } close() {} }\n\
         globalThis.fetch = async (url, init) => { requested.push({ url: String(url), method: init.method }); return new Response(JSON.stringify({ items: ['anvil'] }), { headers: { 'content-type': 'application/json' } }) }\n\
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
         ticks.onmessage({ data: JSON.stringify({ sku: 'anvil', level: 3 }), lastEventId: '' })\n\
         socket.readyState = 1\n\
         socket.onopen?.()\n\
         await chat.sayHello()\n\
         socket.onmessage({ data: JSON.stringify({ text: 'welcome' }) })\n\
         topic.onmessage({ data: JSON.stringify({ sku: 'rope', level: 7 }), lastEventId: '1' })\n\
         console.log(JSON.stringify({ requested, loaded, urls: opened.map((source) => source.url).sort(), ticks: stream.ticks, sent: socket.sent, messages: chat.messages, records: inventory.records }))\n\
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

    async fn report() -> Option<serde_json::Value> {
        if !crate::node_driver::NodeDriver::available() {
            eprintln!("skipping: node is unavailable");
            return None;
        }
        Some(ScriptNamedNeeds::run(&Self::FILES, &Self::driver()).await)
    }
}

#[tokio::test]
async fn a_typescript_request_param_reaches_the_sent_url() {
    if let Some(report) = TypeScriptReach::report().await {
        assert_eq!(
            report["requested"],
            json!([{"url": "http://127.0.0.1:4000/api/products?mode=short", "method": "GET"}]),
            "{report}"
        );
        assert_eq!(report["loaded"], json!({"items": ["anvil"]}), "{report}");
    }
}

#[tokio::test]
async fn a_typescript_stream_request_delivers_its_events_to_a_listener() {
    if let Some(report) = TypeScriptReach::report().await {
        assert!(
            report["urls"]
                .as_array()
                .is_some_and(|urls| urls.contains(&json!("/ticks"))),
            "{report}"
        );
        assert_eq!(
            report["ticks"],
            json!([{"sku": "anvil", "level": 3}]),
            "{report}"
        );
    }
}

#[tokio::test]
async fn a_typescript_socket_request_sends_and_receives_messages() {
    if let Some(report) = TypeScriptReach::report().await {
        assert!(
            report["urls"]
                .as_array()
                .is_some_and(|urls| urls.contains(&json!("ws://127.0.0.1:4000/chat"))),
            "{report}"
        );
        assert_eq!(
            report["sent"],
            json!(["{\"text\":\"Hello from a tab\"}"]),
            "{report}"
        );
        assert_eq!(report["messages"], json!([{"text": "welcome"}]), "{report}");
    }
}

#[tokio::test]
async fn a_typescript_topic_model_is_heard_from_its_topic_stream() {
    if let Some(report) = TypeScriptReach::report().await {
        assert!(
            report["urls"]
                .as_array()
                .is_some_and(|urls| urls.contains(&json!("/.tachyon/topics/inventory-record"))),
            "{report}"
        );
        assert_eq!(
            report["records"],
            json!([{"sku": "rope", "level": 7}]),
            "{report}"
        );
    }
}
