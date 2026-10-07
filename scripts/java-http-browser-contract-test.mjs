#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { chromium, firefox, webkit } from 'playwright'
import { SupervisedCommand } from './supervised-command.mjs'
import { withServer } from './release/server-probe.mjs'

class JavaHttpContract {
  static sourcePin = 'c6e3b77b60eb851dd49d4fa4c940957a9284baea'
  static launchers = { chromium, firefox, webkit }
  static suiteMilliseconds = 900_000
  static projectMilliseconds = 240_000
  static probeMilliseconds = 180_000
  static pageMilliseconds = 20_000
  static files = {
    'client/manifest.json': '{}\n',
    'client/pages/tac.html': `<main>
  <button id="increment" @click="counter.increment()">Add one</button>
  <output id="count">{counter.count}</output>
  <button id="ask" @click="greeting.ask()">Ask Yon</button>
  <button id="clear" @click="greeting.clear()">Clear</button>
  <button id="override" @click="greeting.askCustom()">Use adapter</button>
  <output id="message">{greeting.message}</output>
  <output id="loads">{greeting.loads}</output>
  <output id="clears">{greeting.clears}</output>
  <output id="custom">{greeting.custom}</output>
</main>\n`,
    'client/services/CounterService.java': `@Service
class CounterService {
    int count = 0;
    void increment() { count++; }
}
`,
    'server/models/Greeting.ts': 'export interface Greeting { message: string }\n',
    'server/controllers/GreetingController.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/greeting")
@Public
export default class GreetingController {
  /** @returns {Promise<Greeting>} */
  @GetRoute
  async getGreeting() {
    await new Promise(resolve => setTimeout(resolve, 250));
    return { message: 'Hello from Yon' };
  }
}
`,
    'server/controllers/ClearController.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/clear")
@Public
export default class ClearController {
  /** @returns {Greeting} */
  @PostRoute
  postClear() { return { message: 'Cleared' }; }
}
`,
    'server/controllers/WrongController.js': `@RequestRoute("/api/wrong")
@Public
export default class WrongController {
  /** @returns {string[]} */
  @GetRoute
  getWrong() { return ['private-response-must-not-escape']; }
}
`,
    'server/controllers/FailedClearController.js': `/** @import { Greeting } from '/models/Greeting.ts' */
@RequestRoute("/api/failed-clear")
@Public
export default class FailedClearController {
  /** @returns {Greeting} */
  @PostRoute
  postFailedClear() { throw new Error('private-server-detail-must-not-escape'); }
}
`,
    'client/models/Greeting.java': 'record Greeting(String message) {}\n',
    'client/setups/GreetingSetup.java': `@Setup
class GreetingSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration();
    }
}
`,
    'client/setups/OverrideSetup.java': `@Setup
class OverrideSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(java.util.Map.of(),
            java.util.Map.of("override", (request, current) -> java.util.Map.of("message", "Custom adapter wins")),
            java.util.Map.of(), null);
    }
}
`,
    'client/remotes/GreetingClient.java': `@HTTPClient({"path":"/api","setup":"GreetingSetup"})
@CacheSettings({"name":"java-greetings","ttlMs":60000,"maxBytes":4096,"maxEntries":8})
interface GreetingClient {
    @Cached
    @GetRequest("/greeting")
    Greeting read();
    @ClearCache({"method":"read"})
    @PostRequest("/clear")
    Greeting clear();
    @ClearCache({"method":"read"})
    @PostRequest("/failed-clear")
    Greeting failedClear();
    @GetRequest("/wrong")
    Greeting wrong();
}
`,
    'client/remotes/OverrideClient.java': `@HTTPClient({"path":"/api","adapter":"override","setup":"OverrideSetup"})
interface OverrideClient {
    @GetRequest("/never")
    Greeting read();
}
`,
    'client/services/GreetingService.java': `@Service
class GreetingService {
    @Needs private final GreetingClient greetingClient;
    @Needs private final OverrideClient overrideClient;
    String message = "";
    String custom = "";
    int loads = 0;
    int clears = 0;
    void ask() { loads++; message = greetingClient.read().message(); }
    void clear() { greetingClient.clear(); clears++; }
    void askCustom() { custom = overrideClient.read().message(); }
    Greeting answer() { return greetingClient.read(); }
    Greeting wrong() { return greetingClient.wrong(); }
    void failedClear() { greetingClient.failedClear(); }
}
`,
    'client/tests/GreetingTestSetup.java': JavaHttpContract.testSetup('GreetingTestSetup', 'Test adapter wins'),
    'client/tests/FakeTestSetup.java': JavaHttpContract.testSetup('FakeTestSetup', 'Assigned Fake wins'),
    'client/tests/GreetingServiceTest.java': `@Test(GreetingTestSetup.class, {"partition":"java-http-tests"})
class GreetingServiceTest {
    @Fake GreetingClient greetingClient = null;
    @Fake OverrideClient overrideClient = null;
    void testEmptyFakeUsesTypedTestAdapter() {
        GreetingService service = construct(GreetingService.class);
        service.ask();
        Assert.equals("Test adapter wins", service.message);
        service.askCustom();
        Assert.equals("Test override", service.custom);
    }
    void testAssignedFakeIsCapturedByEachConstruction() {
        Behaviors.Context fake = Behaviors.create("FakeTestSetup", new Behaviors.Options("assigned-fake", new Behaviors.Signal()));
        try {
            greetingClient = new GreetingClient(fake);
            GreetingService first = construct(GreetingService.class);
            greetingClient = null;
            GreetingService second = construct(GreetingService.class);
            first.ask(); second.ask();
            Assert.equals("Assigned Fake wins", first.message);
            Assert.equals("Test adapter wins", second.message);
        } finally { fake.close(); }
    }
}
`,
    'client/tests/CounterServiceTest.java': `@Test(GreetingTestSetup.class, {"partition":"java-counter-tests"})
class CounterServiceTest {
    void testIncrementAddsOne() {
        CounterService counter = new CounterService();
        counter.increment();
        Assert.equals(1, counter.count);
    }
}
`,
  }
  static controls = {
    'client/manifest.json': '{}\n',
    'client/pages/tac.html': '<button id="legacy" @click="legacy.load()">Explicit adapter</button><output id="legacy-message">{legacy.message}</output>\n',
    'client/remotes/LegacyClient.java': `@HTTPClient({"path":"/api","adapter":"legacy","setup":"LegacySetup"})
interface LegacyClient {
    @GetRequest("/never")
    Object read();
}
`,
    'client/setups/LegacySetup.java': `@Setup
class LegacySetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration(java.util.Map.of(),
            java.util.Map.of("legacy", (request, current) -> java.util.Map.of("message", "Legacy adapter wins")),
            java.util.Map.of(), null);
    }
}
`,
    'client/services/LegacyService.java': `@Service
class LegacyService {
    private Behaviors.Context behaviors;
    private LegacyClient client;
    String message = "";
    @OnStart
    void open() {
        behaviors = Behaviors.create("LegacySetup", new Behaviors.Options("legacy", new Behaviors.Signal()));
        client = new LegacyClient(behaviors);
    }
    @PreDestroy
    void close() { behaviors.close(); }
    void load() { message = (String) ((java.util.Map<?, ?>) client.read()).get("message"); }
}
`,
    'client/services/ChoiceService.java': `@Service
class ChoiceService {
    @Needs private final java.util.function.Supplier<String> port;
    String answer() { return port.get(); }
}
`,
    'client/tests/ControlTestSetup.java': `@TestSetup
class ControlTestSetup {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        return new Behaviors.Configuration();
    }
}
`,
    'client/tests/ChoiceServiceTest.java': `@Test(ControlTestSetup.class, {"partition":"java-controls"})
class ChoiceServiceTest {
    @Fake java.util.function.Supplier<String> port;
    void testAssignedFakeIsCaptured() {
        port = () -> "first";
        ChoiceService first = construct(ChoiceService.class);
        port = () -> "second";
        ChoiceService second = construct(ChoiceService.class);
        Assert.equals("first", first.answer());
        Assert.equals("second", second.answer());
    }
    void testUnassignedOrdinaryFakeIsRefused() {
        port = null;
        String refused = "";
        try { construct(ChoiceService.class); }
        catch (IllegalStateException error) { refused = error.getMessage(); }
        Assert.equals("Test.construct needs an assigned Fake named port.", refused);
    }
    void testTestHasNoBrowserTransport() {
        String refused = "";
        try { behaviors.restClient("missing").get("/api/never").retrieve(); }
        catch (Behaviors.Failure error) { refused = error.code; }
        Assert.equals("http-adapter", refused);
    }
}
`,
  }

  static testSetup(name, message) {
    return `@TestSetup
class ${name} {
    static Behaviors.Configuration configure(Behaviors.Execution execution) {
        Behaviors.CacheAdapter cache = new Behaviors.CacheAdapter() {
            private final java.util.Map<String, Behaviors.Entry> values = new java.util.HashMap<>();
            public Behaviors.Entry get(String key, Behaviors.Execution current) {
                return values.getOrDefault(key, new Behaviors.Entry());
            }
            public void set(String key, Object value, Behaviors.Write write) {
                values.put(key, new Behaviors.Entry(value, write.expiresAt));
            }
            public void delete(String key, Behaviors.Execution current) { values.remove(key); }
        };
        return new Behaviors.Configuration(java.util.Map.of("java-greetings", cache),
            java.util.Map.of("greeting", (request, current) -> java.util.Map.of("message", "${message}"),
                "override", (request, current) -> java.util.Map.of("message", "Test override")),
            java.util.Map.of(), (call, current) -> true);
    }
}
`
  }

  static options() {
    const allowed = ['--run', '--worker', '--phase=all', '--phase=controls', '--phase=typed']
    assert.ok(process.argv.slice(2).every(value => allowed.includes(value)), 'unknown fixture option')
    assert.ok(process.argv.includes('--run'), 'Future runtime fixture: explicitly pass --run after the coordinator grants its build/browser lane')
    const phases = process.argv.filter(value => value.startsWith('--phase='))
    assert.ok(phases.length <= 1, 'choose one fixture phase')
    const phase = phases[0]?.slice('--phase='.length) ?? 'all'
    const binary = process.env.TAC_BIN
    const output = process.env.TAC_JAVA_HTTP_ARTIFACT_ROOT
    assert.ok(binary && path.isAbsolute(binary), 'TAC_BIN must name the coordinator-qualified compiler')
    assert.ok(output && path.isAbsolute(output), 'TAC_JAVA_HTTP_ARTIFACT_ROOT must name an external retained output folder')
    assert.ok(process.env.TAC_COMPILER_SOURCE_SHA?.match(/^[a-f0-9]{40}$/), 'record the actual compiler source SHA in TAC_COMPILER_SOURCE_SHA')
    return { phase, binary, output }
  }

  static async record(output, name, value) {
    await writeFile(path.join(output, `${name}.json`), `${JSON.stringify(value, null, 2)}\n`)
  }

  static async command(options, project, label, args) {
    const result = await SupervisedCommand.run(options.binary, args, {
      cwd: project, timeoutMilliseconds: JavaHttpContract.projectMilliseconds,
      maxBuffer: 4 * 1024 * 1024, failOnOutputLimit: true,
    })
    await writeFile(path.join(options.output, `${label}.log`), result.stdout + result.stderr)
    await JavaHttpContract.record(options.output, `${label}-receipt`, {
      command: [options.binary, ...args], cwd: project, ...result,
      stdout: undefined, stderr: undefined, error: result.error?.message,
    })
    assert.equal(result.cleanupConfirmed, true, `${label}: family cleanup must be confirmed`)
    assert.equal(result.timedOut, false, `${label}: timed out`)
    assert.equal(result.error, undefined, `${label}: ${result.error?.message}`)
    assert.equal(result.status, 0, `${label}: ${result.stderr || result.stdout}`)
    return result.stdout + result.stderr
  }

  static async project(options, name, files) {
    const project = await mkdtemp(path.join(options.output, `${name}-`))
    const sources = {}
    for (const [relative, source] of Object.entries(files)) {
      const target = path.join(project, relative)
      await mkdir(path.dirname(target), { recursive: true })
      await writeFile(target, source)
      sources[relative] = createHash('sha256').update(source).digest('hex')
    }
    await JavaHttpContract.record(options.output, `${name}-sources`, { project, sources })
    return project
  }

  static async module(page, name) {
    return await page.evaluate(name => {
      const entry = performance.getEntriesByType('resource').find(entry => new URL(entry.name).pathname.endsWith(`/${name}.js`))
      if (!entry) throw new Error(`Compiled ${name} module missing`)
      return entry.name
    }, name)
  }

  static async typed(page, context, origin, requests) {
    await context.addCookies([{ name: 'private_session', value: 'must-not-travel', url: origin }])
    await page.addInitScript(() => {
      globalThis.javaHttpCalls = []
      globalThis.javaHttpTicks = 0
      globalThis.javaHttpAborts = 0
      const fetch = globalThis.fetch
      globalThis.fetch = (input, options) => {
        const url = String(input)
        if (url.includes('/api/')) {
          globalThis.javaHttpCalls.push({ url, cache: options?.cache, credentials: options?.credentials })
          options?.signal?.addEventListener('abort', () => { globalThis.javaHttpAborts++ }, { once: true })
        }
        return fetch(input, options)
      }
      setInterval(() => { globalThis.javaHttpTicks++ }, 10)
    })
    await page.goto(origin)
    await page.locator('#increment').click()
    await page.waitForFunction(() => document.querySelector('#count')?.textContent === '1')
    const before = await page.evaluate(() => globalThis.javaHttpTicks)
    await page.locator('#ask').click()
    await page.waitForFunction(() => document.querySelector('#message')?.textContent === 'Hello from Yon')
    assert.equal(await page.locator('#loads').textContent(), '1', 'first-app service runs once')
    assert.ok(await page.evaluate(() => globalThis.javaHttpTicks) - before >= 5, 'Java HTTP suspends while the page remains responsive')
    const greetings = () => requests.filter(request => new URL(request.url()).pathname === '/api/greeting')
    assert.equal(greetings().length, 1)
    assert.equal((await greetings()[0].allHeaders()).cookie, undefined, 'same-origin cookies omitted')
    await page.locator('#ask').click()
    await page.waitForFunction(() => document.querySelector('#loads')?.textContent === '2')
    assert.equal(greetings().length, 1, 'Cached typed answer avoids a second network request')
    await page.locator('#override').click()
    await page.waitForFunction(() => document.querySelector('#custom')?.textContent === 'Custom adapter wins')
    assert.ok(!requests.some(request => new URL(request.url()).pathname === '/api/never'), 'explicit adapter wins')
    const module = await JavaHttpContract.module(page, 'GreetingService')
    const missing = await page.evaluate(async module => {
      const Service = (await import(module)).default
      const symbol = Symbol.for('tachyon.tac.http')
      const endpoint = globalThis[symbol]
      const target = new Service({})
      delete globalThis[symbol]
      try { await target.wrong(); return 'completed' }
      catch (error) { return `${error.code ?? ''} ${error}` }
      finally { globalThis[symbol] = endpoint; target.__tachyonDispose() }
    }, module)
    assert.match(missing, /HTTP_MISSING/, 'missing browser host is refused without installing a substitute')
    const error = await page.evaluate(async module => {
      const Service = (await import(module)).default
      const target = new Service({})
      try { await target.wrong(); return { completed: true } }
      catch (error) { return { code: error.code ?? String(error), text: String(error) } }
      finally { target.__tachyonDispose() }
    }, module)
    assert.match(error.code, /HTTP_RESPONSE/, 'typed JSON mismatch is bounded')
    assert.ok(!error.text.includes('private-response-must-not-escape'), 'raw response cannot escape')
    await page.locator('#clear').click()
    await page.waitForFunction(() => document.querySelector('#clears')?.textContent === '1')
    await page.locator('#ask').click()
    await page.waitForFunction(() => document.querySelector('#loads')?.textContent === '3')
    const calls = await page.evaluate(() => globalThis.javaHttpCalls)
    assert.equal(calls.filter(call => new URL(call.url, origin).pathname === '/api/greeting').at(-1).cache, 'no-cache', 'successful ClearCache causes revalidation')
    assert.ok(calls.every(call => call.credentials === 'omit'))
    const typed = await page.evaluate(async module => {
      const Service = (await import(module)).default
      const target = new Service({})
      try { return await target.answer() } finally { target.__tachyonDispose() }
    }, module)
    assert.deepEqual(typed, { message: 'Hello from Yon' }, 'declared record crosses the task answer bridge')
    const failed = await page.evaluate(async module => {
      const Service = (await import(module)).default
      const target = new Service({})
      try {
        await target.answer()
        const count = () => globalThis.javaHttpCalls.filter(call => new URL(call.url, location.origin).pathname === '/api/greeting').length
        const before = count()
        let text = ''
        try { await target.failedClear() } catch (error) { text = String(error) }
        const answer = await target.answer()
        return { text, answer, extraReads: count() - before }
      } finally { target.__tachyonDispose() }
    }, module)
    assert.match(failed.text, /HTTP_STATUS/, 'failed mutation is a bounded HTTP failure')
    assert.ok(!failed.text.includes('private-server-detail-must-not-escape'))
    assert.deepEqual(failed.answer, { message: 'Hello from Yon' })
    assert.equal(failed.extraReads, 0, 'failed mutation preserves the successful cached read')
    await JavaHttpContract.disposal(page, module)
  }

  static async disposal(page, module) {
    const aborts = await page.evaluate(() => globalThis.javaHttpAborts)
    const report = await page.evaluate(async module => {
      const Service = (await import(module)).default
      const first = new Service({})
      const second = new Service({})
      const replacement = new Service({})
      let changes = 0
      let firstDisposed = false
      first.__tachyonStoredChanged = () => { changes++ }
      try {
        await first.clear()
        const pending = first.ask()
        const independent = second.ask()
        if (typeof pending?.then !== 'function') throw new Error('HTTP service method must return a page promise')
        const cancelled = pending.then(() => 'completed', error => `${error.code ?? ''} ${error}`)
        await new Promise(resolve => setTimeout(resolve, 25))
        first.__tachyonDispose()
        firstDisposed = true
        const afterDispose = changes
        const cancellation = await cancelled
        await independent
        let stale = ''
        try { await first.ask() } catch (error) { stale = String(error) }
        await replacement.ask()
        await new Promise(resolve => setTimeout(resolve, 350))
        return { cancellation, stale, second: second.message, replacement: replacement.message, lateChanges: changes - afterDispose }
      } finally {
        if (!firstDisposed) first.__tachyonDispose()
        second.__tachyonDispose()
        replacement.__tachyonDispose()
      }
    }, module)
    assert.match(report.cancellation, /CANCELLED/, 'disposal cancels pending owner HTTP')
    assert.match(report.stale, /Stale native owner|disposed|CANCELLED|aborted/i, 'captured stale owner is refused')
    assert.equal(report.second, 'Hello from Yon', 'another owner stays live')
    assert.equal(report.replacement, 'Hello from Yon', 'replacement owner stays live')
    assert.equal(report.lateChanges, 0, 'disposed owner receives no late changes')
    assert.ok(await page.evaluate(() => globalThis.javaHttpAborts) > aborts, 'owner disposal aborts a real fetch signal')
  }

  static async browser(launcher, origin, phase) {
    const browser = await launcher.launch({ timeout: JavaHttpContract.pageMilliseconds })
    try {
      const context = await browser.newContext()
      const page = await context.newPage()
      page.setDefaultTimeout(JavaHttpContract.pageMilliseconds)
      page.setDefaultNavigationTimeout(JavaHttpContract.pageMilliseconds)
      const errors = []
      const requests = []
      page.on('pageerror', error => errors.push(error.message))
      page.on('request', request => { if (new URL(request.url()).pathname.startsWith('/api/')) requests.push(request) })
      if (phase === 'controls') {
        await page.goto(origin)
        await page.locator('#legacy').click()
        await page.waitForFunction(() => document.querySelector('#legacy-message')?.textContent === 'Legacy adapter wins')
        assert.equal(requests.length, 0, 'legacy Object adapter does not fetch')
      } else await JavaHttpContract.typed(page, context, origin, requests)
      assert.deepEqual(errors, [], 'no unexpected page errors')
      await context.close()
      return { phase, passed: true, httpRequests: requests.map(request => new URL(request.url()).pathname) }
    } finally { await browser.close() }
  }

  static async phase(options, phase) {
    const files = phase === 'controls' ? JavaHttpContract.controls : JavaHttpContract.files
    const testProject = await JavaHttpContract.project(options, `${phase}-test`, files)
    const tests = await JavaHttpContract.command(options, testProject, `${phase}-tests`, ['test', testProject])
    const cases = phase === 'controls'
      ? ['testAssignedFakeIsCaptured', 'testUnassignedOrdinaryFakeIsRefused', 'testTestHasNoBrowserTransport']
      : ['testEmptyFakeUsesTypedTestAdapter', 'testAssignedFakeIsCapturedByEachConstruction', 'testIncrementAddsOne']
    for (const name of cases) assert.match(tests, new RegExp(`  ok  [^\\n]*${name}`), `genuine Test case must execute: ${name}`)
    assert.match(tests, /3 passed, 0 failed\./, 'all three authored cases execute')
    const browserFiles = Object.fromEntries(Object.entries(files).filter(([name]) => !name.startsWith('client/tests/') && name !== 'client/services/ChoiceService.java'))
    const project = await JavaHttpContract.project(options, `${phase}-browser`, browserFiles)
    await JavaHttpContract.command(options, project, `${phase}-bundle`, ['bundle', project, '--no-incremental'])
    const rows = await withServer(options.binary,
      ['serve', project, '--host', '127.0.0.1', '--port', '0', '--no-watch', '--no-self-restart', '--no-bundle'],
      async request => {
        const origin = new URL((await request('/')).url).origin
        const rows = []
        for (const [engine, launcher] of Object.entries(JavaHttpContract.launchers)) {
          const report = { engine, ...await JavaHttpContract.browser(launcher, origin, phase) }
          await JavaHttpContract.record(options.output, `${phase}-${engine}`, report)
          rows.push(report)
          console.log(`${engine}: Java ${phase} passed`)
        }
        return rows
      }, { timeoutMilliseconds: JavaHttpContract.probeMilliseconds, maxBuffer: 4 * 1024 * 1024, failOnOutputLimit: true })
    return { phase, testProject, project, rows }
  }

  static async run() {
    const options = JavaHttpContract.options()
    if (!process.argv.includes('--worker')) {
      await mkdir(options.output)
      const script = fileURLToPath(import.meta.url)
      const result = await SupervisedCommand.run(process.execPath, [script, '--run', '--worker', `--phase=${options.phase}`], {
        timeoutMilliseconds: JavaHttpContract.suiteMilliseconds, maxBuffer: 4 * 1024 * 1024, failOnOutputLimit: true,
      })
      await writeFile(path.join(options.output, 'suite.log'), result.stdout + result.stderr)
      await JavaHttpContract.record(options.output, 'suite-receipt', {
        fixtureSourcePin: JavaHttpContract.sourcePin, compilerSource: process.env.TAC_COMPILER_SOURCE_SHA,
        scriptSha256: createHash('sha256').update(await readFile(script)).digest('hex'),
        compilerSourceIsCallerSupplied: true, phase: options.phase, runtimeAttempted: true,
        acceptanceScope: options.phase === 'all' ? 'complete-contract' : 'diagnostic-stage',
        qualified: options.phase === 'all' && result.status === 0 && result.cleanupConfirmed && !result.timedOut && !result.error,
        ...result,
        stdout: undefined, stderr: undefined, error: result.error?.message,
      })
      assert.equal(result.cleanupConfirmed, true, 'suite family cleanup must be confirmed')
      assert.equal(result.timedOut, false, 'suite timed out')
      assert.equal(result.error, undefined, result.error?.message)
      assert.equal(result.status, 0, result.stderr || result.stdout)
      console.log(result.stdout)
      return
    }
    const phases = options.phase === 'all' ? ['controls', 'typed'] : [options.phase]
    const rows = []
    for (const phase of phases) rows.push(await JavaHttpContract.phase(options, phase))
    await JavaHttpContract.record(options.output, 'results', {
      fixtureSourcePin: JavaHttpContract.sourcePin, compilerSource: process.env.TAC_COMPILER_SOURCE_SHA,
      defaultBudgets: true, rows,
    })
  }
}

await JavaHttpContract.run()
