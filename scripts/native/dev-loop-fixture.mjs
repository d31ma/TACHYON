import { mkdirSync, writeFileSync } from 'node:fs'
import { createServer } from 'node:net'
import path from 'node:path'

export class DevLoopFixture {
  static application = 'DevLoop'
  static identifier = 'dev.tachyon.dev-loop'
  static styleFile = 'client/components/loop/panel/tac.css'
  static pageFile = 'client/components/loop/panel/tac.html'
  static manifestFile = 'client/manifest.json'

  static files = {
    'client/manifest.json': JSON.stringify({
      name: 'Dev Loop',
      short_name: 'Loop',
      start_url: '/',
      tac: {
        application: { id: DevLoopFixture.identifier },
        visibility: { check: false },
      },
    }, null, 2),
    'client/components/loop/panel/tac.css': 'main::after {\n  content: "Style one";\n}\n',
    'client/pages/tac.html': '<loop-panel hydrate="load" />\n',
    'client/components/loop/panel/tac.html': '<main>\n  <h1>Dev loop ready</h1>\n  <button on:click="counter.add()">Add one</button>\n  <p>{counter.label}</p>\n  <button on:click="greeting.ask()">Ask Yon</button>\n  <p>{greeting.text}</p>\n</main>\n',
    'client/models/Greeting.ts': 'export type Greeting = { text: string }\n',
    'client/remotes/GreetingClient.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@HTTPClient('/api/greeting')
export default class GreetingClient {
  /** @returns {Promise<Greeting>} */
  @GetRequest
  async read() {}
}
`,
    'client/services/CounterService.js': `@Service
export default class CounterService {
  taps = 0;
  label = 'Count 0';

  add() {
    this.taps += 1;
    this.label = \`Count \${this.taps}\`;
  }
}
`,
    'client/services/GreetingService.js': `@Service
export default class GreetingService {
  /** @type {GreetingClient} */
  @Needs
  greetingClient;
  text = '';

  async ask() {
    this.text = (await this.greetingClient.read()).text;
  }
}
`,
    'server/models/Greeting.ts': 'export type Greeting = { text: string }\n',
    'server/controllers/GreetingController.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/greeting")
@Public
class GreetingController {
  /** @returns {Greeting} */
  @GetRoute
  getGreeting() {
    return { text: 'Yon says hello' }
  }
}
`,
  }

  static steps = [
    ['wait', ['Dev loop ready', 'Count 0', 'Style one']],
    ['click', 'Add one'],
    ['wait', ['Count 1']],
    ['clickUntil', ['Ask Yon', 'Yon says hello']],
    ['edit', [DevLoopFixture.styleFile, 'Style one', 'Style two']],
    ['wait', ['Style two', 'Count 1']],
    ['edit', [DevLoopFixture.pageFile, 'Dev loop ready', 'Dev loop edited']],
    ['wait', ['Dev loop edited']],
    ['edit', [DevLoopFixture.manifestFile, '"Loop"', '"Loop Two"']],
    ['opened', 2],
    ['wait', ['Dev loop edited', 'Count 0', 'Style two']],
  ]

  /** @param {string} root */
  static write(root) {
    for (const [name, text] of Object.entries(this.files)) {
      const target = path.join(root, name)
      mkdirSync(path.dirname(target), { recursive: true })
      writeFileSync(target, text)
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
}
