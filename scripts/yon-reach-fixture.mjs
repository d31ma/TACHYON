import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export class YonReachFixture {
  static repository = fileURLToPath(new URL('..', import.meta.url))
  static features = ['sockets', 'realtime']
  static elements = ['chat-room', 'ticks-stream', 'note-panel', 'signal-panel', 'entry-panel', 'account-panel']
  static files = {
    'client/models/Note.ts': 'export type Note = { text: string }\n',
    'client/components/note/panel/tac.html': '<button on:click="note.load()">Load note</button>\n<button on:click="note.save()">Save note</button>\n<p id="note">{note.text}</p>\n',
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
    'client/services/NoteService.js': `/** @import { Note } from '/models/Note.ts' */
@Service
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
    'server/models/Note.ts': 'export type Note = { text: string }\n',
    'client/models/ReachSignal.js': `/**
 * @Topic
 * @typedef {Object} ReachSignal
 * @property {string} text
 */
export {}
`,
    'client/components/signal/panel/tac.html': '<button on:click="signal.send()">Send topic</button>\n<p id="topic">{signal.received}</p>\n',
    'client/remotes/SignalClient.js': `/** @import { ReachSignal } from '/models/ReachSignal.js' */
@HTTPClient('/api/signals')
export default class SignalClient {
  /**
   * @param {ReachSignal} signal
   * @returns {Promise<ReachSignal>}
   */
  @PostRequest
  async send(@RequestBody signal) {}
}
`,
    'client/services/SignalService.js': `@Service
export default class SignalService {
  /** @type {SignalClient} */
  @Needs
  signalClient;
  received = '';

  async send() {
    await this.signalClient.send({ text: 'Topic from Yon' });
  }

  /** @param {ReachSignal} signal */
  @OnEvent
  receive(signal) {
    this.received = signal.text;
  }
}
`,
    'server/models/ReachSignal.js': `/**
 * @Topic
 * @typedef {Object} ReachSignal
 * @property {string} text
 */
export {}
`,
    'server/controllers/SignalController.js': `/** @import { ReachSignal } from '/models/ReachSignal.js' */
@RequestRoute("/api/signals")
@Public
class SignalController {
  /** @type {ApplicationEventPublisher} */
  @Needs
  applicationEventPublisher;

  /**
   * @param {ReachSignal} signal
   * @returns {ReachSignal}
   */
  @PostRoute
  postSignal(@FromBody signal) {
    this.applicationEventPublisher.publishEvent(signal)
    return signal
  }
}
`,
    'client/models/Entry.ts': 'export type Entry = { name: string }\n',
    'client/components/entry/panel/tac.html': '<button on:click="entry.sendInvalid()">Send invalid entry</button>\n<p id="entry">{entry.text}</p>\n',
    'client/remotes/EntryClient.js': `/** @import { Entry } from '/models/Entry.ts' */
@HTTPClient('/api/entries')
export default class EntryClient {
  /**
   * @param {Entry} entry
   * @returns {Promise<Entry>}
   */
  @PostRequest
  async write(@RequestBody entry) {}
}
`,
    'client/services/EntryService.js': `/** @import { Entry } from '/models/Entry.ts' */
@Service
export default class EntryService {
  /** @type {EntryClient} */
  @Needs
  entryClient;
  text = '';

  async sendInvalid() {
    this.text = await this.#writeOrRefuse({ name: '' });
  }

  /**
   * @param {Entry} entry
   * @returns {Promise<string>}
   */
  async #writeOrRefuse(entry) {
    try {
      return 'Entry accepted: ' + (await this.entryClient.write(entry)).name;
    } catch {
      return 'Entry refused by Yon';
    }
  }
}
`,
    'server/models/Entry.ts': 'export type Entry = { name: string }\n',
    'server/models/EntryInput.js': `@Checked
export default class EntryInput {
  /** @type {string} */
  @NotEmpty
  name;
}
`,
    'server/services/EntryCheckService.js': `/** @import { Entry } from '/models/Entry.ts' */
@Service
export default class EntryCheckService {
  /**
   * @param {EntryInput} input
   * @returns {Entry}
   */
  accept(@Check input) {
    return /** @type {Entry} */ (input);
  }
}
`,
    'server/controllers/EntryController.js': `/** @import { Entry } from '/models/Entry.ts' */
@RequestRoute("/api/entries")
@Public
class EntryController {
  /** @type {EntryCheckService} */
  @Needs
  entryCheckService;

  /**
   * @param {Entry} entry
   * @returns {Entry}
   */
  @PostRoute
  postEntry(@FromBody entry) {
    return this.entryCheckService.accept(entry)
  }
}
`,
    'client/models/Account.ts': 'export type Account = { user: string }\n',
    'client/components/account/panel/tac.html': '<button on:click="account.visitWithoutToken()">Call without token</button>\n<button on:click="account.visitWithToken()">Call with token</button>\n<p id="account">{account.text}</p>\n',
    'client/remotes/AccountClient.js': `/** @import { Account } from '/models/Account.ts' */
@HTTPClient('/api/account')
export default class AccountClient {
  /** @returns {Promise<Account>} */
  @GetRequest
  async readWithoutToken() {}

  /**
   * @param {string} authorization
   * @returns {Promise<Account>}
   */
  @GetRequest
  async read(@RequestHeader("Authorization") authorization) {}
}
`,
    'client/services/AccountService.js': `@Service
export default class AccountService {
  /** @type {AccountClient} */
  @Needs
  accountClient;
  static SIGNED_IN_AS = 'Signed in as ';
  text = '';

  async visitWithoutToken() {
    this.text = await this.#readOrRefuse();
  }

  async visitWithToken() {
    this.text = AccountService.SIGNED_IN_AS + (await this.accountClient.read('Bearer ada')).user;
  }

  /** @returns {Promise<string>} */
  async #readOrRefuse() {
    try {
      return AccountService.SIGNED_IN_AS + (await this.accountClient.readWithoutToken()).user;
    } catch {
      return 'Refused without a token';
    }
  }
}
`,
    'server/models/Account.ts': 'export type Account = { user: string }\n',
    'server/controllers/ReachIdentityProvider.js': `@IdentityProvider
export class ReachIdentityProvider {
  /** @type {Map<string, { user: string, roles: string[] }>} */
  static _SESSIONS = new Map([
    ['Bearer ada', { user: 'ada', roles: [] }],
  ])

  /**
   * @param {Request} request
   * @returns {{ user: string, roles: string[] } | null}
   */
  identify(request) {
    return this._find(String(request.headers.get('authorization')))
  }

  /**
   * @param {string} authorization
   * @returns {{ user: string, roles: string[] } | null}
   */
  _find(authorization) {
    return ReachIdentityProvider._SESSIONS.get(authorization) ?? null
  }
}
`,
    'server/controllers/AccountController.js': `/** @import { Account } from '/models/Account.ts' */
@RequestRoute("/api/account")
@SignedIn
class AccountController {
  /**
   * @param {string} authorization
   * @returns {Account}
   */
  @GetRoute
  getAccount(@FromHeader("authorization") authorization) {
    return { user: authorization.slice('Bearer '.length) }
  }
}
`,
  }

  /**
   * @param {{ directory: string, yonOrigin: string, appOrigins: string[], manifest?: Record<string, any>, files?: Record<string, string>, elements?: string[] }} options
   */
  constructor({ directory, yonOrigin, appOrigins, manifest = {}, files = {}, elements = [] }) {
    this.directory = directory
    this.yonOrigin = yonOrigin
    this.appOrigins = appOrigins
    this.manifest = manifest
    this.extraFiles = files
    this.extraElements = elements
  }

  writeFile(name, text) {
    const target = path.join(this.directory, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  copyFeature(name) {
    const source = path.join(YonReachFixture.repository, 'website/content/features', name)
    const feature = JSON.parse(readFileSync(path.join(source, 'feature.json'), 'utf8'))
    for (const file of feature.files) {
      const selected = file.name.endsWith('.js') || file.name.endsWith('.html') || file.name.includes('/models/') && file.name.endsWith('.ts')
      if (selected && file.name !== 'client/pages/tac.html') this.writeFile(file.name, readFileSync(path.join(source, file.name), 'utf8'))
    }
  }

  manifestText() {
    const tac = { visibility: { check: false }, ...this.manifest.tac, yon: { origin: this.yonOrigin } }
    return JSON.stringify({ ...this.manifest, tac })
  }

  pageText() {
    return [...YonReachFixture.elements, ...this.extraElements].map((element) => `<${element} hydrate="load" />\n`).join('')
  }

  write() {
    for (const feature of YonReachFixture.features) this.copyFeature(feature)
    for (const [name, text] of Object.entries(YonReachFixture.files)) this.writeFile(name, text)
    this.writeFile('client/pages/tac.html', this.pageText())
    this.writeFile('client/manifest.json', this.manifestText())
    this.writeFile('server/application.json', JSON.stringify({ server: { origins: this.appOrigins } }))
    for (const [name, text] of Object.entries(this.extraFiles)) this.writeFile(name, text)
    return this.directory
  }
}
