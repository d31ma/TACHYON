#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { withServer } from '../release/server-probe.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { YonTlsFront } from './yon-tls-front.mjs'

class LinuxYonOriginTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static fixture = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-yon-origin-'))
  static application = 'YonReach'
  static pageOrigin = 'tachyon://app'
  static features = ['sockets', 'realtime']
  static language = process.env.TAC_YON_SERVICES ?? 'javascript'
  static extensions = { javascript: '.js', rust: '.rs' }
  static probeSteps = [
    ['wait', ['Welcome to the room', 'ANVIL-1 · 3']],
    ['click', 'Say hello'],
    ['wait', ['Hello from a tab']],
    ['click', 'Load note'],
    ['wait', ['Note from Yon']],
    ['click', 'Save note'],
    ['wait', ['Saved from another origin']],
    ['click', 'Order without signing in'],
    ['wait', ['Order refused without sign-in']],
    ['click', 'Order signed in'],
    ['wait', ['Ordered chair', 'Heard order chair']],
    ['click', 'Send an empty product'],
    ['wait', ['Product refused by its check']],
    ['click', 'Send a lamp'],
    ['wait', ['Accepted lamp']],
  ]
  static clientFiles = {
    javascript: {
      'client/remotes/NoteClient.js': `/** @import { Note } from '/models/Note.ts' */
@HTTPClient('/api/notes')
export default class NoteClient {
  /** @returns {Promise<Note>} */
  @GetRequest
  async read() {}

  /**
   * @param {Note} note
   * @returns {Promise<Note>}
   */
  @PostRequest
  async write(@RequestBody note) {}
}
`,
      'client/remotes/OrderClient.js': `@HTTPClient('/api/orders')
export default class OrderClient {
  /**
   * @param {string} token
   * @returns {Promise<string[]>}
   */
  @PostRequest
  async place(@RequestHeader("Authorization") token) {}
}
`,
      'client/remotes/ProductClient.js': `/** @import { Product } from '/models/Product.ts' */
@HTTPClient('/api/products')
export default class ProductClient {
  /**
   * @param {Product} product
   * @returns {Promise<Product>}
   */
  @PostRequest
  async send(@RequestBody product) {}
}
`,
      'client/services/NoteService.js': `@Service
export default class NoteService {
  /** @type {NoteClient} */
  @Needs
  noteClient;
  text = '';

  async load() {
    this.text = (await this.noteClient.read()).text;
  }

  async save() {
    this.text = (await this.noteClient.write({ text: 'from another origin' })).text;
  }
}
`,
      'client/services/OrderService.js': `@Service
export default class OrderService {
  /** @type {OrderClient} */
  @Needs
  orderClient;
  status = '';

  async placeUnsigned() {
    this.status = await this.orderClient.place('').then(() => 'Order accepted without sign-in', () => 'Order refused without sign-in');
  }

  async placeSignedIn() {
    this.status = \`Ordered \${(await this.orderClient.place('member-token')).join(' ')}\`;
  }
}
`,
      'client/services/OrderFeedService.js': `/** @import { OrderPlaced } from '/models/OrderPlaced.ts' */
@Service
export default class OrderFeedService {
  heard = '';

  /** @param {OrderPlaced} order */
  @OnEvent
  heardOrder(order) {
    this.heard = \`Heard order \${order.item}\`;
  }
}
`,
      'client/services/ProductService.js': `@Service
export default class ProductService {
  /** @type {ProductClient} */
  @Needs
  productClient;
  status = '';

  async sendEmpty() {
    this.status = await this.productClient.send({ name: '', level: 0 }).then(() => 'Empty product accepted', () => 'Product refused by its check');
  }

  async sendLamp() {
    this.status = \`Accepted \${(await this.productClient.send({ name: 'lamp', level: 3 })).name}\`;
  }
}
`,
    },
    rust: {
      'client/models/Note.rs': '#[derive(Clone, Debug, Default)]\npub struct Note {\n    pub text: String,\n}\n',
      'client/models/OrderPlaced.rs': '#[Topic]\n#[derive(Clone, Debug, Default)]\npub struct OrderPlaced {\n    pub item: String,\n}\n',
      'client/models/Product.rs': '#[derive(Clone, Debug, Default)]\npub struct Product {\n    pub name: String,\n    pub level: i64,\n}\n',
      'client/setups/NoteSetup.rs': '#[Setup]\nstruct NoteSetup {}\nimpl NoteSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n',
      'client/setups/OrderSetup.rs': '#[Setup]\nstruct OrderSetup {}\nimpl OrderSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n',
      'client/setups/ProductSetup.rs': '#[Setup]\nstruct ProductSetup {}\nimpl ProductSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n',
      'client/remotes/NoteClient.rs': `use crate::models::Note;

#[HTTPClient("/api/notes")]
trait NoteClient {
    #[GetRequest]
    async fn read(&self) -> BehaviorResult<Note>;

    #[PostRequest]
    async fn write(&self, #[RequestBody] note: BehaviorValue) -> BehaviorResult<Note>;
}
`,
      'client/remotes/OrderClient.rs': `#[HTTPClient("/api/orders")]
trait OrderClient {
    #[PostRequest]
    async fn place(&self, #[RequestHeader("Authorization")] token: String) -> BehaviorResult<Vec<String>>;
}
`,
      'client/remotes/ProductClient.rs': `use crate::models::Product;

#[HTTPClient("/api/products")]
trait ProductClient {
    #[PostRequest]
    async fn send(&self, #[RequestBody] product: BehaviorValue) -> BehaviorResult<Product>;
}
`,
      'client/services/NoteService.rs': `#[Service]
pub struct NoteService {
    #[Needs]
    note_client: std::sync::Arc<NoteClient>,
    pub text: String,
}

impl NoteService {
    pub async fn load(&mut self) -> BehaviorResult<()> {
        self.text = self.note_client.read().await?.text;
        Ok(())
    }

    pub async fn save(&mut self) -> BehaviorResult<()> {
        let note = BehaviorValue::object([("text", BehaviorValue::from("from another origin"))]);
        self.text = self.note_client.write(note).await?.text;
        Ok(())
    }
}
`,
      'client/services/OrderService.rs': `#[Service]
pub struct OrderService {
    #[Needs]
    order_client: std::sync::Arc<OrderClient>,
    pub status: String,
}

impl OrderService {
    pub async fn place_unsigned(&mut self) -> BehaviorResult<()> {
        let answer = self.order_client.place(String::new()).await;
        self.status = answer.map_or(String::from("Order refused without sign-in"), |_| String::from("Order accepted without sign-in"));
        Ok(())
    }

    pub async fn place_signed_in(&mut self) -> BehaviorResult<()> {
        let items = self.order_client.place(String::from("member-token")).await?;
        self.status = format!("Ordered {}", items.join(" "));
        Ok(())
    }
}
`,
      'client/services/OrderFeedService.rs': `use crate::models::OrderPlaced;

#[Service]
#[derive(Default)]
pub struct OrderFeedService {
    pub heard: String,
}

impl OrderFeedService {
    #[OnEvent]
    fn heard_order(&mut self, order: OrderPlaced) {
        self.heard = format!("Heard order {}", order.item);
    }
}
`,
      'client/services/ProductService.rs': `#[Service]
pub struct ProductService {
    #[Needs]
    product_client: std::sync::Arc<ProductClient>,
    pub status: String,
}

impl ProductService {
    const NAME: &'static str = "name";
    const LEVEL: &'static str = "level";

    pub async fn send_empty(&mut self) -> BehaviorResult<()> {
        let empty = BehaviorValue::object([(Self::NAME, BehaviorValue::from("")), (Self::LEVEL, BehaviorValue::from(0))]);
        let answer = self.product_client.send(empty).await;
        self.status = answer.map_or(String::from("Product refused by its check"), |_| String::from("Empty product accepted"));
        Ok(())
    }

    pub async fn send_lamp(&mut self) -> BehaviorResult<()> {
        let lamp = BehaviorValue::object([(Self::NAME, BehaviorValue::from("lamp")), (Self::LEVEL, BehaviorValue::from(3))]);
        self.status = format!("Accepted {}", self.product_client.send(lamp).await?.name);
        Ok(())
    }
}
`,
    },
  }

  static files = {
    'client/pages/tac.html': '<chat-room hydrate="load" />\n<ticks-stream hydrate="load" />\n<note-panel hydrate="load" />\n<order-panel hydrate="load" />\n<product-panel hydrate="load" />\n',
    'client/models/Note.ts': 'export type Note = { text: string }\n',
    'client/models/OrderPlaced.ts': '/** @Topic */\nexport interface OrderPlaced {\n  item: string\n}\n',
    'client/models/Product.ts': 'export type Product = { name: string, level: number }\n',
    'client/components/note/panel/tac.html': '<button on:click="note.load()">Load note</button>\n<button on:click="note.save()">Save note</button>\n<p>{note.text}</p>\n',
    'client/components/order/panel/tac.html': '<button on:click="order.placeUnsigned()">Order without signing in</button>\n<button on:click="order.placeSignedIn()">Order signed in</button>\n<p>{order.status}</p>\n<p>{orderFeed.heard}</p>\n',
    'client/components/product/panel/tac.html': '<button on:click="product.sendEmpty()">Send an empty product</button>\n<button on:click="product.sendLamp()">Send a lamp</button>\n<p>{product.status}</p>\n',
    'server/controllers/NoteController.js': `/** @import { Note } from '/models/Note.ts' */
@RequestRoute("/api/notes")
@Public
class NoteController {
  /** @returns {Note} */
  @GetRoute
  getNote() {
    return { text: 'Note from Yon' }
  }

  /**
   * @param {Note} note
   * @returns {Note}
   */
  @PostRoute
  postNote(@FromBody note) {
    return { text: \`Saved \${note.text}\` }
  }
}
`,
    'server/controllers/OrderController.js': `/** @import { OrderPlaced } from '/models/OrderPlaced.ts' */
@RequestRoute("/api/orders")
@SignedIn
class OrderController {
  /** @type {ApplicationEventPublisher} */
  @Needs
  applicationEventPublisher;

  /** @returns {string[]} */
  @PostRoute
  postOrder() {
    /** @type {OrderPlaced} */
    const placed = { item: 'chair' };
    this.applicationEventPublisher.publishEvent(placed);
    return ['chair']
  }
}
`,
    'server/controllers/ProductController.js': `/** @import { Product } from '/models/Product.ts' */
@RequestRoute("/api/products")
@Public
class ProductController {
  /** @type {ProductService} */
  @Needs
  productService;

  /**
   * @param {Product} product
   * @returns {Product}
   */
  @PostRoute
  postProduct(@FromBody product) {
    return this.productService.accept(product);
  }
}
`,
    'server/controllers/AppIdentityProvider.js': `@IdentityProvider
export class AppIdentityProvider {
  /** @type {Map<string, { user: string, roles: string[] }>} */
  static _SESSIONS = new Map([['member-token', { user: 'member', roles: [] }]])

  /**
   * @param {Request} request
   * @returns {{ user: string, roles: string[] } | null}
   */
  identify(request) {
    return this._find(String(request.headers.get('authorization')))
  }

  /**
   * @param {string} token
   * @returns {{ user: string, roles: string[] } | null}
   */
  _find(token) {
    return AppIdentityProvider._SESSIONS.get(token) ?? null
  }
}
`,
    'server/services/ProductService.js': `/** @import { Product } from '/models/Product.ts' */
@Service
export default class ProductService {
  /**
   * @param {ProductInput} input
   * @returns {Product}
   */
  accept(@Check input) {
    return /** @type {Product} */ (input);
  }
}
`,
    'server/models/Note.ts': 'export type Note = { text: string }\n',
    'server/models/OrderPlaced.ts': '/** @Topic */\nexport interface OrderPlaced {\n  item: string\n}\n',
    'server/models/Product.ts': 'export interface Product {\n  name: string;\n  level: number;\n}\n',
    'server/models/ProductInput.js': `@Checked
export default class ProductInput {
  /** @type {string} */
  @NotEmpty
  @Length({"min":1,"max":80})
  name;

  /** @type {number} */
  @Required
  @Min(1)
  @Max(9)
  level;
}
`,
  }

  static write(name, text) {
    const target = path.join(this.fixture, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static copyFeature(name) {
    const source = path.join(this.repository, 'website/content/features', name)
    const feature = JSON.parse(readFileSync(path.join(source, 'feature.json'), 'utf8'))
    for (const file of feature.files) {
      const extension = file.name.startsWith('client/') ? this.extensions[this.language] : '.js'
      const selected = file.name.endsWith(extension) || file.name.endsWith('.html') || file.name.includes('/models/') && file.name.endsWith('.ts')
      if (selected && file.name !== 'client/pages/tac.html') this.write(file.name, readFileSync(path.join(source, file.name), 'utf8'))
    }
  }

  static freePort() {
    return new Promise((resolve, reject) => {
      const server = createServer()
      server.once('error', reject)
      server.listen(0, '127.0.0.1', () => {
        const { port } = server.address()
        server.close(() => resolve(port))
      })
    })
  }

  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  static async prepare(yonOrigin) {
    for (const feature of this.features) this.copyFeature(feature)
    for (const [name, text] of Object.entries({ ...this.files, ...this.clientFiles[this.language] })) this.write(name, text)
    this.write('client/manifest.json', JSON.stringify({
      name: 'Yon Reach',
      start_url: '/',
      tac: { application: { id: 'dev.tachyon.yon-reach' }, visibility: { check: false }, yon: { origin: yonOrigin } },
    }))
    this.write('server/application.json', JSON.stringify({ server: { origins: [this.pageOrigin] } }))
    await this.run(this.binary, ['build', this.fixture, '--target', 'linux'], { timeoutMilliseconds: 240000 })
  }

  static async checkYonAnswers(yon) {
    const origin = { origin: this.pageOrigin }
    const listed = await yon('/api/notes', { headers: origin })
    assert.equal(listed.headers.get('access-control-allow-origin'), this.pageOrigin)
    const preflight = await yon('/api/orders', { method: 'OPTIONS', headers: { ...origin, 'access-control-request-method': 'POST', 'access-control-request-headers': 'authorization,content-type' } })
    assert.equal(preflight.status, 204)
    assert.equal(preflight.headers.get('access-control-allow-headers'), 'authorization,content-type')
    const unsigned = await yon('/api/orders', { method: 'POST', headers: origin })
    assert.equal(unsigned.status, 401)
    const refused = await yon('/api/products', { method: 'POST', headers: { ...origin, 'content-type': 'application/json' }, body: JSON.stringify({ name: '', level: 0 }) })
    assert.equal(refused.status, 422)
    assert.equal(refused.headers.get('access-control-allow-origin'), this.pageOrigin)
  }

  static checkPolicy(yonOrigin) {
    const source = readFileSync(path.join(this.fixture, 'dist/linux/project/tachyon_host.c'), 'utf8')
    const socketOrigin = yonOrigin.replace(/^http/, 'ws')
    assert.match(source, new RegExp(`connect-src [^;]*${yonOrigin.replaceAll('.', '\\.')} ${socketOrigin.replaceAll('.', '\\.')}`))
  }

  static async drive() {
    const state = path.join(this.fixture, 'state')
    const app = path.join(this.fixture, 'dist/linux', this.application, 'bin', this.application)
    const probe = path.join(this.repository, 'scripts/linux/yon-origin-probe.py')
    const script = `set -euo pipefail
"$APP" &
host=$!
trap 'kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true' EXIT
timeout 240s python3 "$PROBE" "$STEPS"`
    const env = {
      ...process.env,
      APP: app,
      PROBE: probe,
      STEPS: JSON.stringify(this.probeSteps),
      TAC_PROBE_APPLICATION: this.application,
      XDG_STATE_HOME: state,
      XDG_DATA_HOME: path.join(this.fixture, 'data'),
      GDK_BACKEND: 'x11',
      GTK_A11Y: 'atspi',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    const output = await this.run('xvfb-run', ['--auto-servernum', '--server-args=-screen 0 1024x1400x24', 'dbus-run-session', '--', 'bash', '-c', script], { env, timeoutMilliseconds: 270000 })
    process.stdout.write(output)
  }

  static tlsFiles() {
    const [cert, key] = [process.env.TAC_YON_TLS_CERT, process.env.TAC_YON_TLS_KEY]
    return cert && key ? { cert, key } : null
  }

  static async declaredOrigin(front, yonPort) {
    return front ? `https://localhost:${await front.listen()}` : `http://127.0.0.1:${yonPort}`
  }

  static async main() {
    const yonPort = await this.freePort()
    const files = this.tlsFiles()
    const front = files && new YonTlsFront(files, yonPort)
    const yonOrigin = await this.declaredOrigin(front, yonPort)
    try {
      await this.prepare(yonOrigin)
      this.checkPolicy(yonOrigin)
      await withServer(this.binary, ['serve', this.fixture, '--no-watch', '--port', String(yonPort)], async (yon) => {
        await this.checkYonAnswers(yon)
        await this.drive()
      })
    } finally {
      front?.close()
    }
    console.log(`PASS: the packaged Linux app reached Yon on ${yonOrigin}: HTTP, a preflighted signed-in request, a topic, a @Check 422, a stream and a socket.`)
  }
}

await LinuxYonOriginTest.main()
