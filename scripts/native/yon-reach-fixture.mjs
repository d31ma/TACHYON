import { mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export class YonReachFixture {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static features = ['sockets', 'realtime']
  static docs = {
    'docs/storage/topics': ['server/controllers/InventoryController.js', 'server/models/InventoryRecord.js', 'client/models/InventoryRecord.js'],
    'docs/annotations/validation-yon': ['server/controllers/OrdersController.js', 'server/services/OrderService.js', 'server/models/OrderInput.js'],
    'docs/yon/who-may-call-a-route-in-every-yon-language': ['server/controllers/AppIdentityProvider.js', 'server/models/Identity.ts', 'server/roles/Admin.js'],
  }
  static applicationId = 'dev.tachyon.yon-reach'
  static applicationName = 'YonReach'
  static proofFile = 'tachyon-yon-reach.json'

  static pageFiles = {
    'client/shared/yon-call.js': `/**
 * @param {string} path
 * @param {RequestInit} init
 * @returns {Promise<{status: number, type: string, body: any}>}
 */
export async function yonCall(path, init = {}) {
  const origin = document.querySelector('meta[name="tachyon-yon-origin"]').content
  const response = await fetch(origin + path, { credentials: 'omit', ...init })
  const type = (response.headers.get('content-type') ?? '').split(';')[0]
  const body = type.includes('json') ? await response.json() : await response.text()
  return { status: response.status, type, body }
}
`,
    'client/pages/tac.html': '<chat-room hydrate="load" />\n<ticks-stream hydrate="load" />\n<yon-reach hydrate="load" />\n',
    'client/models/Note.ts': 'export type Note = { text: string }\n',
    'client/components/yon/reach/tac.html': '<p id="yon-phase">{yonReach.phase}</p>\n<p id="yon-proof">{yonReach.proof}</p>\n',
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
  }

  static serverFiles = {
    'server/models/Note.ts': 'export type Note = { text: string }\n',
    'server/models/Seen.ts': 'export type Seen = { origin: string }\n',
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
    'server/controllers/OriginController.js': `/** @import { Seen } from '/models/Seen.ts' */
@RequestRoute("/api/origin")
@Public
class OriginController {
  /**
   * @param {string} origin
   * @returns {Seen}
   */
  @GetRoute
  getOrigin(@FromHeader("origin") origin) {
    return { origin }
  }
}
`,
    'server/controllers/PrivateController.js': `/** @import { Note } from '/models/Note.ts' */
@RequestRoute("/api/private")
@SignedIn
class PrivateController {
  /** @returns {Note} */
  @GetRoute
  getPrivate() {
    return { text: 'Signed in' }
  }
}
`,
  }

  static featureExtensions = { swift: '.js', rust: '.rs' }
  static nativeFiles = {
    rust: {
      'client/models/Note.rs': '#[derive(Clone, Debug, Default)]\npub struct Note {\n    pub text: String,\n}\n',
      'client/remotes/YonNoteClient.rs': 'use crate::models::Note;\n\n#[HTTPClient("/api")]\ntrait YonNoteClient {\n    #[GetRequest("/notes")]\n    async fn read(&self) -> BehaviorResult<Note>;\n}\n',
      'client/setups/YonNoteSetup.rs': '#[Setup]\nstruct YonNoteSetup {}\nimpl YonNoteSetup {\n    fn configure(_execution: &BehaviorExecution) -> BehaviorResult<BehaviorConfiguration> {\n        Ok(BehaviorConfiguration::default())\n    }\n}\n',
      'client/services/YonReachService.rs': `#[Service]
pub struct YonReachService {
    #[Needs]
    yon_note_client: std::sync::Arc<YonNoteClient>,
    pub phase: String,
    pub proof: String,
}

impl YonReachService {
    const PROOF_FILE: &'static str = "${YonReachFixture.proofFile}";

    pub async fn ask_yon(&mut self) -> String {
        let answer = self.yon_note_client.read().await;
        answer.map_or_else(|error| String::from(error.code), |note| note.text)
    }

    pub fn record(&mut self) -> i64 {
        let pid = std::process::id();
        let record = format!("{{\\"phase\\":{},\\"pid\\":{pid},\\"proof\\":{}}}", Self::quoted(&self.phase), Self::quoted(&self.proof));
        let _ = std::fs::write(std::env::temp_dir().join(Self::PROOF_FILE), record);
        i64::from(pid)
    }

    fn quoted(text: &str) -> String {
        format!("\\"{}\\"", text.replace('\\\\', "\\\\\\\\").replace('"', "\\\\\\""))
    }
}
`,
    },
    swift: {
      'client/models/Note.swift': 'import Foundation\n\nstruct Note: Codable {\n    let text: String\n}\n',
      'client/remotes/YonNoteClient.swift': '@HTTPClient("/api")\nprotocol YonNoteClient {\n    @GetRequest("/notes")\n    func read() async throws -> Note\n}\n',
      'client/setups/YonNoteSetup.swift': '@Setup\nfinal class YonNoteSetup {\n    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {\n        BehaviorAdapters()\n    }\n}\n',
      'client/services/YonReachService.swift': `import Foundation

@Service
final class YonReachService {
    @Needs
    private let yonNoteClient: YonNoteClient
    var phase: String = "waiting"
    var proof: String = ""

    func askYon() async -> String {
        do {
            return try await yonNoteClient.read().text
        } catch let error as BehaviorError {
            return error.code
        } catch {
            return "other"
        }
    }

    func record() -> Int {
        let destination = FileManager.default.temporaryDirectory.appendingPathComponent("${YonReachFixture.proofFile}")
        let record: [String: Any] = ["pid": Int(ProcessInfo.processInfo.processIdentifier), "phase": phase, "proof": proof]
        if let bytes = try? JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]) {
            try? bytes.write(to: destination, options: .atomic)
        }
        return Int(ProcessInfo.processInfo.processIdentifier)
    }
}
`,
    },
  }

  /** @param {boolean} native */
  static service(native) {
    return `/** @import { Note } from '/models/Note.ts' */
import { yonCall } from '/shared/yon-call.js'

@Service
export default class YonReachService {
  static native = ${native}
  static attempts = 300
  static pause = 100
  /** @type {NoteClient} */
  @Needs
  noteClient;
  phase = 'waiting'
  proof = ''
  /** @type {string[]} */
  records = []

  /** @param {InventoryRecord} record */
  @OnEvent
  addRecord(record) {
    this.records = [...this.records, record.sku + ' · ' + record.level]
  }

  @OnStart()
  initialize() {
    setTimeout(() => { void this.probe() }, 50)
  }

  /** @param {() => boolean} check @param {string} label */
  async until(check, label) {
    for (let attempt = 0; attempt < YonReachService.attempts; attempt += 1) {
      if (check()) return
      await new Promise((resolve) => setTimeout(resolve, YonReachService.pause))
    }
    throw new Error('Timed out waiting for ' + label)
  }

  /** @param {string} text */
  shows(text) {
    return document.body.textContent.includes(text)
  }

  /** @param {string} path @param {RequestInit} init */
  call(path, init = {}) {
    return yonCall(path, init)
  }

  /** @param {string} path @param {unknown} value */
  posted(path, value) {
    return yonCall(path, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(value) })
  }

  async probe() {
    const proof = { pageOrigin: location.origin }
    try {
      proof.origin = (await this.call('/api/origin')).body.origin
      proof.read = (await this.noteClient.read()).text
      proof.saved = (await this.noteClient.write({ text: 'from another origin' })).text
      await this.until(() => this.shows('Welcome to the room'), 'the socket welcome')
      ;[...document.querySelectorAll('button')].find((button) => button.textContent === 'Say hello').click()
      await this.until(() => this.shows('Hello from a tab'), 'the socket echo')
      proof.socket = 'Hello from a tab'
      await this.until(() => this.shows('ANVIL-1 · 3'), 'the stream')
      proof.stream = 'ANVIL-1 · 3'
      await this.posted('/inventory', { sku: 'LAMP-9', level: 4 })
      await this.until(() => this.records.includes('LAMP-9 · 4'), 'the topic')
      proof.topic = 'LAMP-9 · 4'
      const rejected = await this.posted('/orders', { name: '', level: 0 })
      proof.check = { status: rejected.status, type: rejected.type, title: rejected.body.title }
      proof.signedOut = (await this.call('/api/private')).status
      const signedIn = await this.call('/api/private', { headers: { authorization: 'ada-token' } })
      proof.signedIn = { status: signedIn.status, text: signedIn.body.text }
      if (YonReachService.native) proof.native = await this.askYon()
      this.phase = 'done'
    } catch (error) {
      proof.error = String(error?.message ?? error)
      this.phase = 'error'
    }
    this.proof = JSON.stringify(proof)
    if (YonReachService.native) await this.record()
  }
}
`
  }

  /** @param {string} root @param {string} name @param {string} text */
  static write(root, name, text) {
    const target = path.join(root, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  /** @param {string} root @param {string} name @param {string} services */
  static copyFeature(root, name, services) {
    const source = path.join(this.repository, 'website/content/features', name)
    const feature = JSON.parse(readFileSync(path.join(source, 'feature.json'), 'utf8'))
    for (const file of feature.files) {
      const extension = file.name.startsWith('client/') ? this.featureExtensions[services] : '.js'
      const selected = file.name.endsWith(extension) || file.name.endsWith('.html') || file.name.includes('/models/') && file.name.endsWith('.ts')
      if (selected && file.name !== 'client/pages/tac.html') this.write(root, file.name, readFileSync(path.join(source, file.name), 'utf8'))
    }
  }

  /** @param {string} root */
  static copyDocs(root) {
    for (const [section, files] of Object.entries(this.docs)) {
      for (const name of files) this.write(root, name, readFileSync(path.join(this.repository, 'website/content', section, name), 'utf8'))
    }
  }

  /**
   * @param {string} root
   * @param {{yonOrigin: string, appOrigins: string[], native: boolean, services?: string}} options
   */
  static prepare(root, { yonOrigin, appOrigins, native, services = 'swift' }) {
    for (const feature of this.features) this.copyFeature(root, feature, native ? services : 'swift')
    this.copyDocs(root)
    const files = { ...this.pageFiles, ...this.serverFiles, ...(native ? this.nativeFiles[services] : {}) }
    for (const [name, text] of Object.entries(files)) this.write(root, name, text)
    this.write(root, 'client/services/YonReachService.js', this.service(native))
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const application = { id: this.applicationId, version: '0.0.1', build: 1 }
    this.write(root, 'client/manifest.json', JSON.stringify({ name: this.applicationName, start_url: '/', tac: { application, ...checks, yon: { origin: yonOrigin } } }))
    this.write(root, 'server/application.json', JSON.stringify({ server: { origins: appOrigins } }))
  }

  /**
   * @param {Record<string, unknown>} proof
   * @param {{pageOrigin: string, native: boolean}} expected
   */
  static assertProof(proof, { pageOrigin, native }) {
    if (proof.error) throw new Error(`The page could not reach Yon: ${proof.error}\n${JSON.stringify(proof)}`)
    const wanted = {
      pageOrigin,
      origin: pageOrigin,
      read: 'Note from Yon',
      saved: 'Saved from another origin',
      socket: 'Hello from a tab',
      stream: 'ANVIL-1 · 3',
      topic: 'LAMP-9 · 4',
      check: { status: 422, type: 'application/problem+json', title: 'Invalid input' },
      signedOut: 401,
      signedIn: { status: 200, text: 'Signed in' },
      ...(native ? { native: 'Note from Yon' } : {}),
    }
    for (const [key, value] of Object.entries(wanted)) {
      if (JSON.stringify(proof[key]) !== JSON.stringify(value)) {
        throw new Error(`Yon reach ${key}: expected ${JSON.stringify(value)}, got ${JSON.stringify(proof[key])}\n${JSON.stringify(proof)}`)
      }
    }
  }
}
