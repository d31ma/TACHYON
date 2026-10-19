#!/usr/bin/env python3

import argparse
import json
from pathlib import Path


class YonReachFixture:
    PAGE_ORIGINS = {
        "windows": "https://tachyon.local",
        "macos": "tachyon-app://bundle",
        "ios": "tachyon-app://bundle",
        "android": "https://appassets.tachyon.local",
        "linux": "tachyon://app",
    }
    RUST_TARGETS = ["windows", "macos", "linux"]
    CSHARP_TARGETS = ["windows"]
    TARGETS = ["web", "windows", "macos", "ios", "android", "linux"]

    YON_FILES = {
        "server/models/Greeting.ts": "export interface Greeting {\n  message: string\n}\n",
        "server/models/Tick.ts": "export interface Tick {\n  sku: string\n  level: number\n}\n",
        "server/models/ChatMessage.ts": "export interface ChatMessage {\n  text: string\n}\n",
        "server/models/ReachNews.js": """/**
 * @Topic
 * @typedef {Object} ReachNews
 * @property {string} text
 */
export {}
""",
        "server/models/OrderInput.js": """@Checked
export default class OrderInput {
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
""",
        "server/services/OrderService.js": """@Service
export default class OrderService {
  /**
   * @param {OrderInput} order
   * @returns {OrderInput}
   */
  accept(@Check order) {
    return order;
  }
}
""",
        "server/controllers/HelloController.js": """/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/hello")
@Public
class HelloController {
  /** @returns {Greeting} */
  @GetRoute
  getHello() {
    return { message: 'hello' }
  }
}
""",
        "server/controllers/TicksController.js": """/** @import { Tick } from '/models/Tick.ts' */
@RequestRoute("/api/ticks")
@Public
class TicksController {
  /** @returns {AsyncGenerator<Tick>} */
  @StreamRoute
  async *streamTicks() {
    yield* this._countdown(3)
  }

  /**
   * @param {number} start
   * @returns {AsyncGenerator<Tick>}
   */
  async *_countdown(start) {
    for (let level = start; level > 0; level -= 1) {
      yield { sku: 'REACH', level }
      await new Promise((resume) => setTimeout(resume, 200))
    }
  }
}
""",
        "server/controllers/EchoController.js": """/** @import { ChatMessage } from '/models/ChatMessage.ts' */
@SocketRoute('/api/echo')
@Public
class EchoController {
  /**
   * @param {SocketSession} session
   * @returns {Promise<void>}
   */
  @OnOpen
  onOpen(session) {
    return session.send({ text: 'open' })
  }

  /**
   * @param {SocketSession} session
   * @param {ChatMessage} message
   * @returns {Promise<void>}
   */
  @OnMessage
  onMessage(session, message) {
    return session.send(message)
  }
}
""",
        "server/controllers/NewsController.js": """@RequestRoute("/api/news")
@Public
export class NewsController {
  /** @type {ApplicationEventPublisher} */
  @Needs
  applicationEventPublisher

  /**
   * @param {ReachNews} item
   * @returns {ReachNews}
   */
  @PostRoute
  postNews(@FromBody item) {
    this.applicationEventPublisher.publishEvent(item)
    return item
  }
}
""",
        "server/controllers/OrdersController.js": """@RequestRoute("/api/orders")
@Public
class OrdersController {
  /** @type {OrderService} */
  @Needs
  orderService;

  /**
   * @param {any} order
   * @returns {any}
   */
  @PostRoute
  postOrders(@FromBody order) {
    return this.orderService.accept(order);
  }
}
""",
        "server/controllers/AccountController.js": """/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/account")
@SignedIn
export class AccountController {
  /** @returns {Greeting} */
  @GetRoute
  getAccount() {
    return { message: 'reach' }
  }
}
""",
        "server/controllers/AppIdentityProvider.js": """@IdentityProvider
export class AppIdentityProvider {
  /** @type {Map<string, { user: string, roles: string[] }>} */
  static _SESSIONS = new Map([
    ['Bearer reach', { user: 'reach', roles: [] }],
  ])

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
""",
    }

    PAGE_FILES = {
        "client/pages/tac.html": '<reach-panel hydrate="load" />\n',
        "client/models/Greeting.ts": "export type Greeting = { message: string }\n",
        "client/models/Tick.ts": "export type Tick = { sku: string; level: number }\n",
        "client/models/ChatMessage.ts": "export type ChatMessage = { text: string }\n",
        "client/models/OrderInput.ts": "export type OrderInput = { name: string; level: number }\n",
        "client/models/ReachNews.js": """/**
 * @Topic
 * @typedef {Object} ReachNews
 * @property {string} text
 */
export {}
""",
        "client/remotes/HelloClient.js": """/** @import { Greeting } from '/models/Greeting.ts' */
@HTTPClient('/api/hello')
export default class HelloClient {
  /** @returns {Promise<Greeting>} */
  @GetRequest
  async read() {}
}
""",
        "client/remotes/AccountClient.js": """/** @import { Greeting } from '/models/Greeting.ts' */
@HTTPClient('/api/account')
export default class AccountClient {
  /** @returns {Promise<Greeting>} */
  @GetRequest
  async readAnonymous() {}

  /**
   * @param {string} authorization
   * @returns {Promise<Greeting>}
   */
  @GetRequest
  async readSignedIn(@RequestHeader("Authorization") authorization) {}
}
""",
        "client/remotes/OrdersClient.js": """/** @import { OrderInput } from '/models/OrderInput.ts' */
@HTTPClient('/api/orders')
export default class OrdersClient {
  /**
   * @param {OrderInput} order
   * @returns {Promise<OrderInput>}
   */
  @PostRequest
  async write(@RequestBody order) {}
}
""",
        "client/remotes/NewsClient.js": """@HTTPClient('/api/news')
export default class NewsClient {
  /**
   * @param {ReachNews} item
   * @returns {Promise<ReachNews>}
   */
  @PostRequest
  async post(@RequestBody item) {}
}
""",
        "client/remotes/TicksClient.js": """/** @import { Tick } from '/models/Tick.ts' */
@HTTPClient('/api/ticks')
export default class TicksClient {
  /** @param {Tick} tick */
  @StreamRequest
  streamTicks(tick) {}
}
""",
        "client/remotes/EchoClient.js": """/** @import { ChatMessage } from '/models/ChatMessage.ts' */
@HTTPClient('/api/echo')
export default class EchoClient {
  /** @param {ChatMessage} message */
  @SocketRequest
  receive(message) {}
}
""",
        "client/services/StreamService.js": """/** @import { Tick } from '/models/Tick.ts' */
@Service
export default class StreamService {
  /** @type {TicksClient} */
  @Needs
  ticksClient;
  count = 0
  result = 'Stream waiting'

  /** @param {Tick} tick */
  @OnEvent
  addTick(tick) {
    this.count += 1
    this.result = `Stream ok ${this.count}`
  }
}
""",
        "client/services/SocketService.js": """/** @import { ChatMessage } from '/models/ChatMessage.ts' */
@Service
export default class SocketService {
  static ECHO = 'echo'
  /** @type {EchoClient & { send: SocketSend<ChatMessage> }} */
  @Needs
  echoClient;
  result = 'Socket waiting'

  /** @param {ChatMessage} message */
  @OnEvent
  receive(message) {
    this.result = this.#label(message)
  }

  /** @returns {Promise<void>} */
  check() {
    return this.echoClient.send({ text: SocketService.ECHO })
  }

  /**
   * @param {ChatMessage} message
   * @returns {string}
   */
  #label(message) {
    return message.text === SocketService.ECHO ? 'Socket ok echo' : `Socket ${message.text}`
  }
}
""",
        "client/services/ReachService.js": """/** @import { Greeting } from '/models/Greeting.ts' */
@Service
export default class ReachService {
  static TOKEN = 'Bearer reach'
  static OUTCOMES = { accepted: 'accepted', adapter: 'refused' }
  /** @type {HelloClient} */
  @Needs
  helloClient;
  /** @type {OrdersClient} */
  @Needs
  ordersClient;
  /** @type {NewsClient} */
  @Needs
  newsClient;
  /** @type {AccountClient} */
  @Needs
  accountClient;
  httpResult = 'HTTP waiting'
  topicSent = 'Topic not sent'
  topicResult = 'Topic waiting'
  validResult = 'Check valid waiting'
  checkResult = 'Check invalid waiting'
  signedInResult = 'SignedIn waiting'
  tokenResult = 'Token waiting'
  topicText = ''

  async checkHttp() {
    this.httpResult = `HTTP ${await this.#answer(this.helloClient.read())}`
  }

  async checkTopic() {
    this.topicText = `topic ${Date.now()}`
    const outcome = await this.#settle(this.newsClient.post({ text: this.topicText }))
    this.topicSent = `Topic sent ${this.#describe(outcome)}`
  }

  /** @param {ReachNews} item */
  @OnEvent
  receiveNews(item) {
    this.topicResult = this.#topicLabel(item)
  }

  async checkValidation() {
    const valid = await this.#settle(this.ordersClient.write({ name: 'lamp', level: 3 }))
    this.validResult = `Check valid ${this.#describe(valid)}`
    const invalid = await this.#settle(this.ordersClient.write({ name: '', level: 0 }))
    this.checkResult = `Check invalid ${this.#describe(invalid)}`
  }

  async checkSignedIn() {
    const outcome = await this.#settle(this.accountClient.readAnonymous())
    this.signedInResult = `SignedIn anonymous ${this.#describe(outcome)}`
    this.tokenResult = `SignedIn ${await this.#answer(this.accountClient.readSignedIn(ReachService.TOKEN))}`
  }

  /**
   * @param {ReachNews} item
   * @returns {string}
   */
  #topicLabel(item) {
    return item.text === this.topicText ? 'Topic ok' : this.topicResult
  }

  /**
   * @param {Promise<Greeting>} pending
   * @returns {Promise<string>}
   */
  async #answer(pending) {
    try {
      const greeting = await pending
      return `ok ${greeting.message}`
    } catch (error) {
      return `failed ${error.code}`
    }
  }

  /**
   * @param {Promise<unknown>} pending
   * @returns {Promise<string>}
   */
  async #settle(pending) {
    try {
      await pending
      return 'accepted'
    } catch (error) {
      return String(error.code)
    }
  }

  /**
   * @param {string} outcome
   * @returns {string}
   */
  #describe(outcome) {
    return ReachService.OUTCOMES[outcome] ?? outcome
  }
}
""",
    }

    RUST_FILES = {
        "client/models/Greeting.rs": "pub struct Greeting {\n    pub message: String,\n}\n",
        "client/remotes/RustHelloClient.rs": """use crate::models::Greeting;

#[HTTPClient("/api")]
trait RustHelloClient {
    #[GetRequest("/hello")]
    async fn get_hello(&self) -> BehaviorResult<Greeting>;
}
""",
        "client/setups/RustHelloSetup.rs": """#[Setup]
struct RustHelloSetup {}
impl RustHelloSetup {
    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {
        Ok(BehaviorConfiguration::default())
    }
}
""",
        "client/services/RustReachService.rs": """use crate::models::Greeting;

#[Service]
pub struct RustReachService {
    #[Needs]
    rust_hello_client: std::sync::Arc<RustHelloClient>,
}

impl RustReachService {
    pub async fn ask_server(&mut self) -> String {
        let answer = self.rust_hello_client.get_hello().await;
        Self::message(answer)
    }

    fn message(answer: BehaviorResult<Greeting>) -> String {
        match answer {
            Ok(greeting) => greeting.message,
            Err(error) => format!("failed {}", error.code),
        }
    }
}
""",
    }

    CSHARP_FILES = {
        "client/remotes/CsharpHelloClient.cs": """[HTTPClient({"path":"/api/hello","setup":"CsharpHelloSetup"})]
public interface CsharpHelloClient
{
    [GetRequest]
    System.Threading.Tasks.Task<BehaviorJSON> read();
}
""",
        "client/setups/CsharpHelloSetup.cs": """[Setup]
public sealed class CsharpHelloSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        return new BehaviorAdapters();
    }
}
""",
        "client/services/CsharpReachService.cs": """[Service]
public sealed class CsharpReachService
{
    [Needs]
    private readonly CsharpHelloClient _csharpHelloClient;

    public async System.Threading.Tasks.Task<BehaviorJSON> AskServer()
    {
        return await _csharpHelloClient.read();
    }
}
""",
    }

    @staticmethod
    def linked_twin(service, label):
        source = """@Service
export default class __SERVICE__ {
  static HELLO = 'hello'
  result = '__LABEL__ HTTP waiting'

  async check() {
    const text = this.#text(await this.#ask())
    this.result = `__LABEL__ HTTP ${this.#label(text)}`
  }

  /** @returns {Promise<string | { message: string }>} */
  async #ask() {
    try {
      return await this.askServer()
    } catch (error) {
      return `failed ${error.name}`
    }
  }

  /**
   * @param {string} text
   * @returns {string}
   */
  #label(text) {
    return text === __SERVICE__.HELLO ? `ok ${text}` : text
  }

  /**
   * @param {string | { message: string }} message
   * @returns {string}
   */
  #text(message) {
    return message?.message ?? message
  }
}
"""
        return source.replace("__SERVICE__", service).replace("__LABEL__", label)

    @staticmethod
    def component(linked):
        lines = [
            '<main aria-label="Yon reach">',
            "<h1>Yon reach</h1>",
            '<button on:click="reach.checkHttp()">Check HTTP</button>',
            "<p>{reach.httpResult}</p>",
            "<p>{stream.result}</p>",
            '<button on:click="socket.check()">Check socket</button>',
            "<p>{socket.result}</p>",
            '<button on:click="reach.checkTopic()">Check topic</button>',
            "<p>{reach.topicSent}</p>",
            "<p>{reach.topicResult}</p>",
            '<button on:click="reach.checkValidation()">Check validation</button>',
            "<p>{reach.validResult}</p>",
            "<p>{reach.checkResult}</p>",
            '<button on:click="reach.checkSignedIn()">Check signed in</button>',
            "<p>{reach.signedInResult}</p>",
            "<p>{reach.tokenResult}</p>",
        ]
        for instance, label in linked:
            lines.append(f'<button on:click="{instance}.check()">Check {label} HTTP</button>')
            lines.append(f"<p>{{{instance}.result}}</p>")
        lines.append("</main>")
        return "\n".join(lines) + "\n"

    def __init__(self, root, target, yon_origin, page_origin):
        self.root = root
        self.target = target
        self.yon_origin = yon_origin
        self.page_origin = page_origin

    def write(self, relative, source):
        destination = self.root / relative
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_text(source, encoding="utf-8")

    def write_all(self, files):
        for relative, source in files.items():
            self.write(relative, source)

    def origins(self):
        origins = []
        if self.target in self.PAGE_ORIGINS:
            origins.append(self.PAGE_ORIGINS[self.target])
        if self.page_origin:
            origins.append(self.page_origin)
        if not origins:
            raise RuntimeError("the web target needs --page-origin, the origin that serves the page")
        return origins

    def manifest(self):
        tac = {
            "application": {"id": "dev.tachyon.yon-reach", "version": "0.0.1", "build": 1},
            "visibility": {"check": False},
            "yon": {"origin": self.yon_origin},
        }
        return {"name": "Yon Reach", "start_url": "/", "tac": tac}

    def linked(self):
        services = []
        if self.target in self.RUST_TARGETS:
            self.write_all(self.RUST_FILES)
            services.append(("rustReach", "Rust"))
            self.write("client/services/RustReachService.js", self.linked_twin("RustReachService", "Rust"))
        if self.target in self.CSHARP_TARGETS:
            self.write_all(self.CSHARP_FILES)
            services.append(("csharpReach", "CSharp"))
            self.write("client/services/CsharpReachService.js", self.linked_twin("CsharpReachService", "CSharp"))
        return services

    def create(self):
        if any(self.root.iterdir()):
            raise RuntimeError("the fixture directory must be empty")
        self.write("server/application.json", json.dumps({"server": {"origins": self.origins()}}, indent=2) + "\n")
        self.write("client/manifest.json", json.dumps(self.manifest(), indent=2) + "\n")
        self.write_all(self.YON_FILES)
        self.write_all(self.PAGE_FILES)
        linked = self.linked()
        self.write("client/components/reach/panel/tac.html", self.component(linked))


def main():
    parser = argparse.ArgumentParser(description="Write a project whose page reaches a Yon on another origin.")
    parser.add_argument("directory", type=Path)
    parser.add_argument("target", choices=YonReachFixture.TARGETS)
    parser.add_argument("yon_origin")
    parser.add_argument("--page-origin", default="")
    arguments = parser.parse_args()
    arguments.directory.mkdir(parents=True, exist_ok=True)
    fixture = YonReachFixture(arguments.directory, arguments.target, arguments.yon_origin, arguments.page_origin)
    fixture.create()


if __name__ == "__main__":
    main()
