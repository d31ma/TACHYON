import assert from 'node:assert/strict'
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CachedClientGate } from '../cached-client-browser-test.mjs'
import { WebApiGate } from '../web-apis-browser-test.mjs'

class WasmProbeSource {
  static slots = 32
  static script = 'client/shared/scripts/WasmProbe.js'
  static prelude = `export default class WasmProbe {
  static WAIT = 90000;
  static PAUSE = 50;
  static START = 50;
  static LINE = 400;
  static lines = [];
  static clicked = [];
  static proof = '';

  static phase = 'starting';
  static PAINT = 200;

  static {
    document.addEventListener('click', (event) => { WasmProbe.clicked.push(event.target?.closest?.('button')?.textContent.trim() ?? ''); }, true);
    setInterval(() => WasmProbe.paint(), WasmProbe.PAINT);
    setTimeout(() => { void WasmProbe.run(); }, WasmProbe.START);
  }

  static async run() {
    try {
      this.render('running');
      await this.steps();
      await this.finish('passed');
    } catch (error) {
      this.line('not ok - ' + String(error?.message ?? error));
      await this.finish('failed');
    }
  }

  static paint() {
    const phase = document.querySelector('#wasm-probe-phase');
    if (phase && phase.textContent !== 'Probe ' + this.phase) phase.textContent = 'Probe ' + this.phase;
    const slots = document.querySelectorAll('.wasm-probe-line');
    this.lines.forEach((text, index) => { if (slots[index] && slots[index].textContent !== text) slots[index].textContent = text; });
  }

  /** @param {string} phase */
  static render(phase) {
    this.phase = phase;
    this.paint();
  }

  /** @param {string} text */
  static line(text) {
    this.lines.push(text.slice(0, this.LINE));
    this.paint();
  }

  /** @param {string} phase */
  static async finish(phase) {
    this.proof = JSON.stringify({ check: this.CHECK, language: this.LANGUAGE, phase, lines: this.lines, origin: location.origin });
    this.render(phase);
    const recorder = this.service('wasmRecorder');
    if (typeof recorder?.record === 'function') await recorder.record(this.proof);
  }

  /** @param {string} name */
  static service(name) {
    return globalThis.__tachyonTac?.service(name);
  }

  /** @param {string[]} names */
  static async ready(names) {
    await this.until(() => names.every((name) => this.service(name)), 'the services ' + names.join(', ') + ' to start');
  }

  /** @template T @param {() => T} check @param {string} what @returns {Promise<T>} */
  static async until(check, what) {
    const deadline = Date.now() + this.WAIT;
    let problem = '';
    while (Date.now() < deadline) {
      try {
        const value = await check();
        if (value) return value;
      } catch (error) {
        problem = String(error?.message ?? error);
      }
      await new Promise((resolve) => setTimeout(resolve, this.PAUSE));
    }
    throw new Error('timed out waiting for ' + what + (problem ? ' (' + problem + ')' : '') + '; the page shows: ' + this.pageText().slice(0, 200));
  }

  /** @param {string} text */
  static ok(text) {
    this.line('ok - ' + this.LANGUAGE + ': ' + text);
  }

  /** @param {unknown} actual @param {unknown} wanted @param {string} label */
  static expect(actual, wanted, label) {
    if (JSON.stringify(actual) !== JSON.stringify(wanted)) throw new Error(label + ': expected ' + JSON.stringify(wanted) + ', got ' + JSON.stringify(actual));
    this.ok(label);
  }

  /** @param {() => unknown} read @param {unknown} wanted @param {string} label */
  static async settle(read, wanted, label) {
    await this.until(() => JSON.stringify(read()) === JSON.stringify(wanted), label + ' (' + JSON.stringify(wanted) + ')');
    this.ok(label);
  }

  static pageText() {
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    const parts = [];
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
      if (!node.parentElement?.closest('#wasm-probe')) parts.push(node.textContent);
    }
    return parts.join(' ').replace(/\\s+/g, ' ');
  }

  /** @param {string} text */
  static pageHas(text) {
    return this.pageText().includes(text);
  }

  static items() {
    return [...document.querySelectorAll('li')].filter((item) => !item.closest('#wasm-probe')).map((item) => item.textContent.trim());
  }

  /** @param {string} label @param {ParentNode} [within] */
  static press(label, within = document) {
    const button = [...within.querySelectorAll('button')].find((item) => item.textContent.trim() === label);
    if (!button) throw new Error('no button ' + JSON.stringify(label) + ' on the page');
    button.click();
  }

  /** @param {string} label */
  static async gesture(label) {
    const seen = this.clicked.length;
    this.render('waiting for ' + label);
    await this.until(() => this.clicked.slice(seen).includes(label), 'the host to click ' + label);
    this.render('running');
  }
`

  static view = `<section id="wasm-probe" aria-label="WebAssembly feature probe">
<p id="wasm-probe-phase">Probe starting</p>
${Array.from({ length: WasmProbeSource.slots }, () => '<p class="wasm-probe-line"></p>').join('\n')}
</section>
`

  /** @param {string} check @param {string} language @param {string} steps @param {Record<string, unknown>} [constants] */
  static source(check, language, steps, constants = {}) {
    const values = { CHECK: check, LANGUAGE: language, ...constants }
    const declared = Object.entries(values).map(([name, value]) => `  static ${name} = ${JSON.stringify(value)};`).join('\n')
    const named = `    const { ${Object.keys(values).join(', ')} } = WasmProbe;`
    return `${this.prelude.replace('  static WAIT', `${declared}\n  static WAIT`)}\n  static async steps() {\n${named}\n${steps}\n  }\n}\n`
  }
}

class HeldClientsCheck {
  static languages = ['swift', 'kt', 'cs']
  static features = ['sockets', 'realtime']
  static needsYon = true
  static ticks = ['ANVIL-1 · 3', 'ANVIL-1 · 2', 'ANVIL-1 · 1']
  static page = '<chat-room hydrate="load" />\n<ticks-stream hydrate="load" />\n<wasm-probe />\n'
  static steps = `    await this.until(() => this.pageHas('Welcome to the room'), 'the socket welcome');
    this.ok("the service's socket client opened with its component and heard the welcome");
    this.press('Say hello');
    await this.until(() => this.pageHas('Hello from a tab'), 'the socket to echo the message');
    this.ok("the service's send wrote to the socket and the page heard the message back");
    for (const tick of TICKS) await this.until(() => this.pageHas(tick), 'the stream tick ' + tick);
    this.ok("the service's stream client heard every tick of the stream");`

  /** @param {string} name @param {string} language */
  static chosen(name, language) {
    const server = name.startsWith('server/') && (name.endsWith('.js') || name.includes('/models/') && name.endsWith('.ts'))
    const client = name.startsWith('client/') && name.endsWith(`.${language}`)
    return server || client
  }

  /** @param {string} project @param {string} language */
  static stage(project, language) {
    for (const feature of this.features) {
      const source = path.join(WasmFeaturesFixture.features, feature)
      cpSync(path.join(source, 'client/components'), path.join(project, 'client/components'), { recursive: true })
      const { files } = JSON.parse(readFileSync(path.join(source, 'feature.json'), 'utf8'))
      for (const { name } of files.filter((file) => this.chosen(file.name, language))) WasmFeaturesFixture.write(project, name, readFileSync(path.join(source, name), 'utf8'))
    }
    WasmFeaturesFixture.files(project, { 'client/pages/tac.html': this.page })
    return { constants: { TICKS: this.ticks } }
  }
}

class YonHttpCheck {
  static languages = ['swift', 'kt', 'cs']
  static needsYon = true
  static examples = fileURLToPath(new URL('../../website/content/docs/calling-the-server/', import.meta.url))
  static folders = { swift: 'browser-http-swift', kt: 'browser-http-kotlin', cs: 'browser-http-csharp' }
  static steps = `    const greeting = () => this.service('greeting');
    const origin = document.querySelector('meta[name="tachyon-yon-origin"]')?.content;
    await this.ready(['greeting']);
    this.press('Ask Yon');
    await this.until(() => this.pageHas('Hello from Yon'), 'the answer from Yon');
    this.ok('a WebAssembly HTTPClient read the answer of a Yon route on tac.yon.origin ' + origin);
    if (this.pageHas('Clear cached answer')) {
      this.press('Ask Yon');
      await this.until(() => Number(greeting().loads) === 2, 'the second read');
      this.press('Clear cached answer');
      await this.until(() => Number(greeting().clears) === 1, 'the ClearCache call to Yon');
      this.press('Ask Yon');
      await this.until(() => Number(greeting().loads) === 3 && this.pageHas('Hello from Yon'), 'the read after ClearCache');
      this.ok('a Cached read, a ClearCache POST to Yon and a fresh read answered in the app');
    }
    if (this.pageHas('Use custom adapter')) {
      this.press('Use custom adapter');
      await this.until(() => this.pageHas('Custom adapter wins'), "the Setup's http adapter");
      this.ok("a Setup's http adapter answered a WebAssembly HTTPClient inside the app");
    }`

  /** @param {string} project @param {string} language */
  static stage(project, language) {
    const example = path.join(this.examples, this.folders[language])
    cpSync(path.join(example, 'client'), path.join(project, 'client'), { recursive: true })
    cpSync(path.join(example, 'server'), path.join(project, 'server'), { recursive: true })
    const page = readFileSync(path.join(example, 'client/pages/tac.html'), 'utf8')
    WasmFeaturesFixture.files(project, { 'client/pages/tac.html': `${page.trimEnd()}\n<wasm-probe />\n` })
    return { tac: { budgets: { check: false } } }
  }
}

class WebApisCheck {
  static languages = ['swift', 'kt', 'cs']
  static needsYon = false
  static classes = { swift: 'SwiftStampService', kt: 'KtStampService', cs: 'CsStampService' }
  static unitMethods = ['kt', 'cs', 'fs']
  static ownRuntimes = ['cs', 'fs']
  static gestures = ['Copy from the service', 'Paste into the service', 'Ask to notify', 'Share a link']
  static browserApis = ['clipboard', 'share', 'notifications', 'geolocation']
  static refusal = 'The clipboard browser API needs a user action. Call it from a click or key handler.'
  static permissions = { geolocation: 'The probe reads where you are', notifications: 'The probe asks to notify you' }
  static steps = `    const stamp = () => this.service(LANGUAGE + 'Stamp');
    await this.ready([LANGUAGE + 'Stamp']);
    this.expect(navigator.userActivation?.isActive, false, 'the page has no user action before any click');
    this.expect(stamp().attempt(), REFUSAL, 'Clipboard refuses a call without a user action');
    const before = Date.now();
    const now = stamp().now();
    this.expect(now >= before && now <= Date.now(), true, 'Clock gives the time of the web view');
    this.expect(stamp().zone(), Intl.DateTimeFormat().resolvedOptions().timeZone, 'Clock gives the time zone of the web view');
    this.expect(/^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(stamp().token()), true, 'SecureRandom gives a version 4 UUID');
    this.expect(stamp().count(), 16, 'SecureRandom gives the bytes it was asked for');
    this.expect(stamp().motion(), matchMedia('(prefers-reduced-motion: reduce)').matches ? 'reduce' : 'no-preference', 'Preferences gives the motion setting of the web view');
    this.expect(stamp().locale(), navigator.language, 'Preferences gives the language of the web view');
    await this.settle(() => stamp().scheme, matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light', 'a replay PreferencesChanged listener heard the color scheme');
    await this.settle(() => stamp().online, navigator.onLine, 'a replay ConnectivityChanged listener heard the connectivity');
    if (UNIT_METHOD) {
      this.expect(stamp().forget() ?? null, null, 'a method that returns nothing answers nothing');
    }
    if (OWN_RUNTIME) {
      const started = Date.now();
      stamp().nap();
      this.expect(Date.now() - started >= 20, true, "the language's own sleep makes the page wait");
      this.expect(stamp().year(), new Date().getUTCFullYear(), "the language's own clock gives the year");
      this.expect(stamp().roll() >= 1 && stamp().roll() <= 6, true, "the language's own random numbers work");
    }
    if (GESTURES.length) {
      await this.gesture(GESTURES[0]);
      await this.gesture(GESTURES[1]);
      await this.until(() => stamp().pasted === 'from ' + LANGUAGE || stamp().failure, 'a clipboard answer');
      this.expect({ pasted: stamp().pasted, failure: stamp().failure }, { pasted: 'from ' + LANGUAGE, failure: '' }, 'Clipboard wrote text from a click and ClipboardTextRead brought it back to the service');
      await this.gesture(GESTURES[2]);
      await this.until(() => stamp().permission, 'a notification permission answer');
      this.ok('NotificationPermissionChanged reached the service: ' + stamp().permission);
      await this.gesture(GESTURES[3]);
      await this.until(() => stamp().shared === true || stamp().failure !== '', 'a share answer');
      this.ok('Share answered the service: ' + (stamp().shared === true ? 'ShareFinished' : 'BrowserCallFailed ' + stamp().failure));
    }`

  /** @param {string} language */
  static source(language) {
    const name = `client/services/${this.classes[language]}.${language}`
    const source = language === 'cs' ? WebApiGate.csharpStamp : WebApiGate.files[name]
    assert.ok(source, `scripts/web-apis-browser-test.mjs has no ${language} stamp service; set up the ${language} toolchain it looks for`)
    return { [name]: source }
  }

  /** @param {string} project @param {string} language @param {{ gestures?: boolean }} options */
  static stage(project, language, { gestures = true } = {}) {
    const stamp = `${language}Stamp`
    const calls = [`copy('from ${language}')`, 'paste()', 'ask()', 'tell()']
    const view = `<main><p>{${stamp}.label}</p>\n${calls.map((call, index) => `<button on:click="${stamp}.${call}">${this.gestures[index]}</button>`).join('\n')}\n</main>\n`
    WasmFeaturesFixture.files(project, { ...this.source(language), 'client/components/stamp/page/tac.html': view, 'client/pages/tac.html': '<stamp-page hydrate="load" />\n<wasm-probe />\n' })
    return {
      tac: { browserApis: this.browserApis, application: { permissions: this.permissions } },
      constants: { REFUSAL: this.refusal, UNIT_METHOD: this.unitMethods.includes(language), OWN_RUNTIME: this.ownRuntimes.includes(language), GESTURES: gestures ? this.gestures : [] },
      gestures: gestures ? this.gestures : [],
    }
  }
}

class DocumentTitleCheck {
  static languages = ['kt']
  static needsYon = false
  static fixture = fileURLToPath(new URL('../../crates/tachyon-core/tests/fixtures/tac-document-title/client', import.meta.url))
  static harness = `<main aria-label="DocumentTitle in the app">
  <button on:click="harness.second = true">Show second Kotlin owner</button>
  <button on:click="harness.first = false">Dispose first Kotlin owner</button>
  <logic :if="harness.first">
    <section id="kotlin-first-container"><title-kotlin :label="'Kotlin first — 🚀'" :fails="false" /></section>
  </logic>
  <logic :if="harness.second">
    <section id="kotlin-second-container"><title-kotlin :label="'Kotlin second — 雪'" :fails="false" /></section>
  </logic>
</main>
`
  static steps = `    const first = () => document.querySelector('#kotlin-first-container');
    const title = (wanted, label) => this.settle(() => document.title, wanted, label);
    await title('Kotlin first — 🚀', "the first Kotlin owner set document.title in its OnStart");
    this.press('Write Kotlin title', first());
    await title('Kotlin click — 🌍', 'a Kotlin method set the title from a click');
    this.press('Attempt Kotlin title', first());
    await title('Kotlin attempt — 雪', 'a guarded Kotlin call set the title');
    this.press('Clear Kotlin title', first());
    await title('', 'an empty Kotlin title cleared document.title');
    this.press('Show second Kotlin owner');
    await title('Kotlin second — 雪', 'a second Kotlin owner set its title when it mounted');
    this.press('Dispose first Kotlin owner');
    await this.until(() => !first(), 'the first owner to unmount');
    await title('Kotlin second — 雪', "disposing the first owner left the second owner's title");`

  /** @param {string} project */
  static stage(project) {
    const copied = ['services/KotlinTitleService.kt', 'services/HarnessService.js', 'components/title/kotlin/tac.html']
    WasmFeaturesFixture.files(project, {
      ...Object.fromEntries(copied.map((name) => [`client/${name}`, readFileSync(path.join(this.fixture, name), 'utf8')])),
      'client/components/title/harness/tac.html': this.harness,
      'client/pages/tac.html': '<title-harness />\n<wasm-probe />\n',
    })
    return {}
  }
}

class CachedClientCheck {
  static languages = ['swift', 'kt', 'cs']
  static needsYon = false
  static folders = ['remotes', 'services', 'setups']
  static steps = `    const stamp = () => this.service(LANGUAGE + 'Stamp');
    await this.ready([LANGUAGE + 'Stamp']);
    void stamp().load();
    await this.until(() => stamp().loads === 1, 'the first read');
    const first = stamp().stamp;
    this.expect(String(first).includes('stamp'), true, "an HTTPClient read answered through its Setup's adapter: " + first);
    void stamp().load();
    await this.until(() => stamp().loads === 2, 'the second read');
    this.expect(stamp().stamp, first, 'a Cached HTTPClient read answered from its cache');`

  /** @param {string} project @param {string} language */
  static stage(project, language) {
    const chosen = Object.entries(CachedClientGate.files).filter(([name]) => this.folders.includes(name.split('/')[1]) && name.endsWith(`.${language}`))
    assert.ok(chosen.length, `scripts/cached-client-browser-test.mjs has no ${language} stamp client`)
    WasmFeaturesFixture.files(project, { ...Object.fromEntries(chosen), 'client/components/stamp/page/tac.html': `<main><p>{${language}Stamp.stamp}</p></main>\n`, 'client/pages/tac.html': '<stamp-page hydrate="load" />\n<wasm-probe />\n' })
    return {}
  }
}

class AsyncListenerCheck {
  static languages = ['swift', 'kt', 'cs', 'fs']
  static needsYon = false
  static example = fileURLToPath(new URL('../../website/content/docs/cookbook/signal-driven-ui/client', import.meta.url))
  static names = ['Ada', 'Lin']
  static steps = `    for (const [index, name] of NAMES.entries()) {
      const field = await this.until(() => document.querySelector('input[aria-label="Name"]'), 'the name field');
      field.value = name;
      field.dispatchEvent(new Event('input', { bubbles: true }));
      await this.until(() => this.service('userForm')?.name === name, 'the form service to hold ' + name);
      this.press('Save');
      await this.until(() => this.items().length >= index + 1, 'the list to show ' + name);
    }
    this.expect(this.items(), NAMES, 'an async OnEvent listener reloaded every saved name through its client');`

  /** @param {string} project @param {string} language */
  static stage(project, language) {
    cpSync(path.join(this.example, 'components'), path.join(project, 'client/components'), { recursive: true })
    for (const folder of ['models', 'remotes', 'services', 'setups']) {
      for (const file of readdirSync(path.join(this.example, folder)).filter((name) => name.endsWith(`.${language}`))) {
        cpSync(path.join(this.example, folder, file), path.join(project, 'client', folder, file))
      }
    }
    WasmFeaturesFixture.files(project, { 'client/pages/tac.html': '<users-form hydrate="load" />\n<users-list hydrate="load" />\n<wasm-probe />\n' })
    return { constants: { NAMES: this.names } }
  }
}

class PageProgramCheck {
  static languages = ['swift', 'kt', 'java', 'dart', 'cs', 'fs', 'hpp']
  static needsYon = false
  static fixtures = fileURLToPath(new URL('../page-program/', import.meta.url))
  static folders = { swift: 'swift', kt: 'kotlin', java: 'java', dart: 'dart', cs: 'csharp', fs: 'fsharp', hpp: 'cpp' }
  static steps = `    const writer = () => this.service('writer');
    const reader = () => this.service('reader');
    await this.ready(['writer', 'reader']);
    await writer().inspect();
    await reader().inspect();
    await this.until(() => writer().identity && reader().identity, "both services to read the Setup's adapter");
    this.expect(writer().identity === reader().identity, true, 'the two services share one Setup configuration, one program per page');
    await writer().increment();
    await this.until(() => Number(writer().count) === 1, "the writer's increment");
    await reader().inspect();
    await this.until(() => Number(reader().count) === 1, "the reader to see the writer's increment");
    this.ok("the reader saw the writer's increment through its own client");
    await writer().remember('hello');
    await reader().recall();
    await this.until(() => reader().recalled === 'hello', "the reader's store to find the writer's record");
    this.ok("the reader's Store found the record the writer's Store saved");`

  /** @param {string} project @param {string} language */
  static stage(project, language) {
    cpSync(path.join(this.fixtures, 'shared'), project, { recursive: true })
    cpSync(path.join(this.fixtures, this.folders[language], 'client'), path.join(project, 'client'), { recursive: true })
    WasmFeaturesFixture.files(project, { 'client/pages/tac.html': '<writer-panel hydrate="load" />\n<reader-panel hydrate="load" />\n<wasm-probe />\n' })
    return {}
  }
}

export class WasmFeaturesFixture {
  static features = fileURLToPath(new URL('../../website/content/features', import.meta.url))
  static checks = {
    'held-clients': HeldClientsCheck,
    'yon-http': YonHttpCheck,
    'web-apis': WebApisCheck,
    'document-title': DocumentTitleCheck,
    'cached-client': CachedClientCheck,
    'async-listener': AsyncListenerCheck,
    'page-program': PageProgramCheck,
  }
  static linked = { linux: ['rs'], macos: ['swift', 'rs'], ios: ['swift', 'rs'], android: ['kt', 'rs'], windows: ['cs', 'rs'] }
  static pageOrigins = ['tachyon://app', 'tachyon-app://bundle', 'https://appassets.tachyon.local', 'https://tachyon.local']
  static extensions = { swift: 'Swift', kt: 'Kotlin', cs: 'Csharp', fs: 'Fsharp', java: 'Java', dart: 'Dart', hpp: 'Cpp', rs: 'Rust' }
  static checkFlags = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
  static idleYon = 'http://127.0.0.1:9'
  static passed = 'Probe passed'
  static failed = 'Probe failed'

  /** @param {string} project @param {string} name @param {string} text */
  static write(project, name, text) {
    const target = path.join(project, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  /** @param {string} project @param {Record<string, string>} files */
  static files(project, files) {
    for (const [name, text] of Object.entries(files)) this.write(project, name, text)
  }

  /** The languages of a check that run as WebAssembly in the given host's web view. @param {string} check @param {string} target */
  static languages(check, target) {
    const linked = this.linked[target]
    assert.ok(linked, `unknown target ${target}; use ${Object.keys(this.linked).join(', ')}`)
    return this.checks[check].languages.filter((language) => !linked.includes(language))
  }

  /** @param {string} check @param {string} language */
  static application(check, language) {
    const words = check.split('-').map((word) => word[0].toUpperCase() + word.slice(1)).join('')
    return `Wasm${words}${this.extensions[language]}`
  }

  /**
   * Stages one check for one language as a native app project.
   * @param {string} project @param {string} check @param {string} language
   * @param {{ yonOrigin?: string, gestures?: boolean, recorder?: Record<string, string> }} [options]
   */
  static stage(project, check, language, options = {}) {
    const fixture = this.checks[check]
    assert.ok(fixture, `unknown check ${check}; use ${Object.keys(this.checks).join(', ')}`)
    assert.ok(fixture.languages.includes(language), `the ${check} check has no ${language} fixture; it has ${fixture.languages.join(', ')}`)
    assert.ok(!fixture.needsYon || options.yonOrigin, `the ${check} check needs a running Yon: pass yonOrigin`)
    const staged = fixture.stage(project, language, options)
    const application = this.application(check, language)
    const id = `dev.tachyon.wasm-${check}-${language}`
    const yon = { yon: { origin: fixture.needsYon ? options.yonOrigin : this.idleYon } }
    const tac = { ...this.checkFlags, ...staged.tac, scripts: [`/${WasmProbeSource.script.slice('client/'.length)}`], application: { ...staged.tac?.application, id, version: '0.0.1' }, ...yon }
    this.write(project, 'client/manifest.json', JSON.stringify({ name: application, start_url: '/', tac }, null, 2))
    if (fixture.needsYon) this.write(project, 'server/application.json', JSON.stringify({ server: { origins: this.pageOrigins } }))
    this.files(project, {
      [WasmProbeSource.script]: WasmProbeSource.source(check, language, fixture.steps, staged.constants),
      'client/components/wasm/probe/tac.html': WasmProbeSource.view,
      ...options.recorder,
    })
    return { application, id, gestures: staged.gestures ?? [], needsYon: fixture.needsYon }
  }

  /** Every check and language that runs as WebAssembly on the target, narrowed by optional lists. @param {string} target @param {string[]} [checks] @param {string[]} [languages] */
  static plan(target, checks = Object.keys(this.checks), languages = []) {
    return checks.flatMap((check) => this.languages(check, target).filter((language) => !languages.length || languages.includes(language)).map((language) => ({ check, language })))
  }

  /** Asserts the services ship as WebAssembly in the app, not as a linked companion. @param {string} output @param {string} bundle */
  static assertWebAssembly(output, bundle) {
    const host = path.join(output, 'tachyon.host.json')
    const companions = existsSync(host) ? JSON.parse(readFileSync(host, 'utf8')).companions ?? [] : []
    assert.deepEqual(companions.map((item) => `${item.language} ${item.service}`), [], 'no service of the check is linked into the app')
    const modules = readdirSync(bundle, { recursive: true }).filter((name) => String(name).endsWith('.wasm'))
    assert.ok(modules.length, `the app ships no WebAssembly module under ${bundle}`)
    return modules.map(String)
  }

  /** @param {string} proof */
  static assertProof(proof) {
    const record = JSON.parse(proof)
    assert.equal(record.phase, 'passed', `the in-app probe failed:\n${record.lines.join('\n')}`)
    return record
  }
}
