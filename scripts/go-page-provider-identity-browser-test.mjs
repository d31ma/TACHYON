#!/usr/bin/env node
import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdir, readFile, writeFile } from 'node:fs/promises'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { SupervisedCommand } from './supervised-command.mjs'
import { withServer } from './release/server-probe.mjs'

class GoPageProviderIdentity {
  static repository = fileURLToPath(new URL('../', import.meta.url))
  static script = fileURLToPath(import.meta.url)
  static engines = ['chromium', 'firefox', 'webkit']
  static timeout = 15_000
  static commandTimeout = 240_000
  static eventPrefix = 'GO_PAGE_EVENT '
  static twoSameShapedServicesManifest = '{"tac":{"duplication":{"check":false}}}\n'
  static setup = String.raw`package main

import (
	"encoding/json"
	"fmt"
	"sync"
)

// @Setup
type PageIdentitySetup struct {
	lock     sync.Mutex
	identity string
	count    int
}

func (provider *PageIdentitySetup) Configure(execution *BehaviorExecution) (*BehaviorConfiguration, error) {
	identity, err := (SecureRandom{}).UUID()
	if err != nil {
		return nil, err
	}
	provider.identity = identity
	encoded, err := json.Marshal(map[string]any{"kind": "construct", "identity": identity})
	if err != nil {
		return nil, err
	}
	fmt.Printf("GO_PAGE_EVENT %s\n", encoded)
	configuration := NewBehaviorConfiguration()
	configuration.Http["pageidentity"] = provider
	return configuration, nil
}

func (provider *PageIdentitySetup) Exchange(request *BehaviorHttpRequest, execution *BehaviorExecution) (BehaviorValue, error) {
	if err := execution.Check(); err != nil {
		return nil, err
	}
	provider.lock.Lock()
	defer provider.lock.Unlock()
	if request.Path == "/provider/increment" && request.Method == "POST" {
		provider.count++
	}
	return ProviderState{Identity: provider.identity, Count: provider.count, Constructed: 1}, nil
}
`
  static client = String.raw`package main

// @HTTPClient({"adapter":"pageidentity","path":"/provider","setup":"PageIdentitySetup"})
type PageIdentityClient interface {
	// @GetRequest("/read")
	Read() (ProviderState, error)
	// @PostRequest("/increment")
	Increment() (ProviderState, error)
}
`
  static model = String.raw`package main

type ProviderState struct {
	Identity    string ` + '`json:"identity"`' + String.raw`
	Count       int    ` + '`json:"count"`' + String.raw`
	Constructed int    ` + '`json:"constructed"`' + String.raw`
}
`

  static service(role) {
    const className = `${role[0].toUpperCase()}${role.slice(1)}Service`
    return String.raw`package main

import (
	"encoding/json"
	"errors"
	"fmt"
)

// @Service
type ${className} struct {
	context          *BehaviorContext
	client           *PageIdentityClient
	Identity         string
	Count            int
	Constructed      int
	ContextID        string
	ContextDisposals int
}

// @OnStart
func (service *${className}) Start() error { return service.Open() }

func (service *${className}) Open() error {
	context, err := service.connectWhenClosed()
	contextID, identityErr := service.contextIdentity(context)
	kept := service.keptContext(context, identityErr)
	service.attach(kept, contextID)
	return errors.Join(err, identityErr)
}

func (service *${className}) connectWhenClosed() (*BehaviorContext, error) {
	if service.context != nil {
		return nil, nil
	}
	return BehaviorsCreate[PageIdentitySetup](NewBehaviorOptions("public-${role}", NewBehaviorSignal()))
}

func (service *${className}) contextIdentity(context *BehaviorContext) (string, error) {
	if context == nil {
		return "", nil
	}
	return (SecureRandom{}).UUID()
}

func (service *${className}) keptContext(context *BehaviorContext, failure error) *BehaviorContext {
	if failure != nil {
		context.Dispose()
		return nil
	}
	return context
}

func (service *${className}) attach(context *BehaviorContext, contextID string) {
	if context == nil {
		return
	}
	service.context = context
	service.client = NewPageIdentityClientWith(context)
	service.ContextID = contextID
}

func (service *${className}) Inspect() error {
	state, err := service.client.Read()
	service.saveWhenRead(state, err)
	return err
}

func (service *${className}) Increment() error {
	state, err := service.client.Increment()
	service.saveWhenRead(state, err)
	return err
}

func (service *${className}) saveWhenRead(state ProviderState, failure error) {
	if failure != nil {
		return
	}
	service.Identity = state.Identity
	service.Count = state.Count
	service.Constructed = state.Constructed
}

func (service *${className}) Release() {
	released := service.disposeWhenOpen()
	encoded, err := service.disposalEvent(released)
	service.stopOn(err)
	service.printEvent(encoded)
}

func (service *${className}) disposeWhenOpen() bool {
	if service.context == nil {
		return false
	}
	service.context.Dispose()
	service.context.Dispose()
	service.context = nil
	service.ContextDisposals++
	return true
}

func (service *${className}) disposalEvent(released bool) ([]byte, error) {
	if !released {
		return nil, nil
	}
	return json.Marshal(map[string]any{"kind": "dispose-context", "role": "${role}", "context": service.ContextID, "identity": service.Identity, "disposals": service.ContextDisposals})
}

func (service *${className}) stopOn(failure error) {
	if failure != nil {
		panic(failure)
	}
}

func (service *${className}) printEvent(encoded []byte) {
	if encoded == nil {
		return
	}
	fmt.Printf("GO_PAGE_EVENT %s\n", encoded)
}

// @OnStop
func (service *${className}) Stop() { service.Release() }
`
  }

  static panel(role) {
    return `<section id="${role}-panel"><h2>${role}</h2>
<button id="${role}-read" on:click="${role}.inspect()">Read ${role}</button>
<button id="${role}-increment" on:click="${role}.increment()">Increment ${role}</button>
<button id="${role}-open" on:click="${role}.open()">Open ${role}</button>
<button id="${role}-release" on:click="${role}.release()">Release ${role}</button>
<p id="${role}-identity">{${role}.identity}</p><p id="${role}-count">{${role}.count}</p>
<p id="${role}-constructed">{${role}.constructed}</p><p id="${role}-disposals">{${role}.contextDisposals}</p></section>\n`
  }

  static sources() {
    return {
      'client/manifest.json': GoPageProviderIdentity.twoSameShapedServicesManifest,
      'client/pages/tac.html': '<provider-scope />\n',
      'client/pages/control/tac.html': '<writer-panel />\n',
      'client/components/provider/scope/tac.html': '<main><button id="hide" on:click="visibility.show = false">Hide both consumers</button><button id="show" on:click="visibility.show = true">Show both consumers</button><logic :if="visibility.show"><writer-panel /><reader-panel /></logic></main>\n',
      'client/components/writer/panel/tac.html': GoPageProviderIdentity.panel('writer'),
      'client/components/reader/panel/tac.html': GoPageProviderIdentity.panel('reader'),
      'client/services/VisibilityService.js': '@Service()\nexport default class VisibilityService { show = true; }\n',
      'client/services/WriterService.go': GoPageProviderIdentity.service('writer'),
      'client/services/ReaderService.go': GoPageProviderIdentity.service('reader'),
      'client/setups/PageIdentitySetup.go': GoPageProviderIdentity.setup,
      'client/remotes/PageIdentityClient.go': GoPageProviderIdentity.client,
      'client/models/ProviderState.go': GoPageProviderIdentity.model,
    }
  }

  static option(name) {
    return process.argv.find(argument => argument.startsWith(`${name}=`))?.slice(name.length + 1)
  }

  static required(name) {
    const value = process.env[name]
    assert.ok(value, `${name} must identify the retained run`)
    return value
  }

  static async emit(root) {
    const hashes = {}
    for (const [relative, source] of Object.entries(GoPageProviderIdentity.sources())) {
      const file = path.join(root, relative)
      await mkdir(path.dirname(file), { recursive: true })
      await writeFile(file, source)
      hashes[relative] = createHash('sha256').update(source).digest('hex')
    }
    await writeFile(path.join(root, 'authored-source-sha256.json'), JSON.stringify(hashes, null, 2) + '\n')
    return hashes
  }

  static async build(binary, project, output) {
    const result = await SupervisedCommand.run(binary, ['bundle', project, '--target', 'web', '--no-incremental'], { cwd: GoPageProviderIdentity.repository, timeoutMilliseconds: GoPageProviderIdentity.commandTimeout, failOnOutputLimit: true })
    await writeFile(path.join(output, 'bundle.log'), `${result.stdout}\n${result.stderr}\n${result.error?.stack ?? ''}`)
    assert.equal(result.error, undefined, result.error?.message)
    assert.equal(result.cleanupConfirmed, true)
    assert.equal(result.status, 0, 'a bundle/setup failure is not an identity RED')
  }

  static async run() {
    const output = path.resolve(GoPageProviderIdentity.required('TAC_GO_PAGE_OUT'))
    const binary = path.resolve(GoPageProviderIdentity.required('TAC_BIN'))
    const sourceCommit = GoPageProviderIdentity.required('TAC_GO_PAGE_SOURCE_COMMIT')
    const chosen = GoPageProviderIdentity.option('--engine')
    const engines = chosen ? [chosen] : GoPageProviderIdentity.engines
    assert.ok(engines.every(engine => GoPageProviderIdentity.engines.includes(engine)), 'unknown engine; nothing was run')
    const project = path.join(output, 'project')
    const hashes = await GoPageProviderIdentity.emit(project)
    const metadata = { sourceCommit, compilerSha256: createHash('sha256').update(await readFile(binary)).digest('hex'), fixtureSha256: createHash('sha256').update(await readFile(GoPageProviderIdentity.script)).digest('hex'), authoredSources: hashes, defaultBudgets: true }
    await writeFile(path.join(output, 'inputs.json'), JSON.stringify(metadata, null, 2) + '\n')
    await GoPageProviderIdentity.build(binary, project, output)
    const failures = []
    await withServer(binary, ['preview', project, '--static', '--out-dir', 'dist/web', '--host', '127.0.0.1', '--port', '0'], async request => {
      const origin = new URL((await request('/')).url).origin
      for (const engine of engines) {
        const result = await SupervisedCommand.run(process.execPath, [GoPageProviderIdentity.script, `--probe=${engine}`, `--origin=${origin}`, `--output=${output}`], { timeoutMilliseconds: GoPageProviderIdentity.commandTimeout, failOnOutputLimit: true })
        await writeFile(path.join(output, `${engine}-process.log`), `${result.stdout}\n${result.stderr}\n${result.error?.stack ?? ''}`)
        if (result.error || !result.cleanupConfirmed || result.status !== 0) failures.push({ engine, status: result.status, cleanupConfirmed: result.cleanupConfirmed, error: result.error?.message })
      }
    }, { cwd: GoPageProviderIdentity.repository, timeoutMilliseconds: GoPageProviderIdentity.commandTimeout * (engines.length + 1) })
    await writeFile(path.join(output, 'run.json'), JSON.stringify({ ...metadata, engines, status: failures.length ? 'fail' : 'pass', failures }, null, 2) + '\n')
    assert.deepEqual(failures, [], 'every selected engine must pass its actual provider assertions')
  }

  static observe(page, name, diagnostics) {
    page.on('pageerror', error => diagnostics.errors.push({ page: name, error: String(error) }))
    page.on('requestfailed', request => diagnostics.failedRequests.push({ page: name, url: request.url() }))
    page.on('console', message => {
      for (const line of message.text().split('\n')) {
        if (line.startsWith(GoPageProviderIdentity.eventPrefix)) diagnostics.events.push({ page: name, ...JSON.parse(line.slice(GoPageProviderIdentity.eventPrefix.length)) })
      }
    })
  }

  static async initial(page, origin) {
    await page.goto(origin, { waitUntil: 'networkidle', timeout: GoPageProviderIdentity.timeout })
    await page.locator('#writer-read').click()
    await page.locator('#reader-read').click()
    await page.waitForFunction(() => document.querySelector('#writer-identity')?.textContent && document.querySelector('#reader-identity')?.textContent, undefined, { timeout: GoPageProviderIdentity.timeout })
    return await GoPageProviderIdentity.state(page)
  }

  static async state(page) {
    return await page.evaluate(() => {
      const writer = __tachyonTac.service('writer')
      const reader = __tachyonTac.service('reader')
      return { writer: { identity: writer.identity, count: writer.count, constructed: writer.constructed, context: writer.contextID, disposals: writer.contextDisposals }, reader: { identity: reader.identity, count: reader.count, constructed: reader.constructed, context: reader.contextID, disposals: reader.contextDisposals } }
    })
  }

  static shared(state) {
    assert.match(state.writer.identity, /^[a-f\d-]{36}$/)
    assert.equal(state.writer.identity, state.reader.identity, 'two real Services must see the same configured provider allocation')
    assert.notEqual(state.writer.context, state.reader.context, 'contexts and their ownership remain separate')
    assert.equal(state.writer.constructed, 1)
    assert.equal(state.reader.constructed, 1)
    assert.equal(state.writer.disposals, 0)
    assert.equal(state.reader.disposals, 0)
  }

  static events(diagnostics, page, kind) {
    return diagnostics.events.filter(event => event.page === page && event.kind === kind)
  }

  static async waitEvents(diagnostics, page, kind, expected) {
    const deadline = Date.now() + GoPageProviderIdentity.timeout
    while (GoPageProviderIdentity.events(diagnostics, page, kind).length < expected && Date.now() < deadline) await new Promise(resolve => setTimeout(resolve, 20))
    assert.equal(GoPageProviderIdentity.events(diagnostics, page, kind).length, expected, `${page} ${kind} counter`)
  }

  static async control(page, origin, diagnostics) {
    diagnostics.phase = 'healthy-single-service-registry-control'
    await page.goto(`${origin}/control/`, { waitUntil: 'networkidle', timeout: GoPageProviderIdentity.timeout })
    await page.locator('#writer-read').click()
    await page.waitForFunction(() => document.querySelector('#writer-identity')?.textContent, undefined, { timeout: GoPageProviderIdentity.timeout })
    const proof = await page.evaluate(async () => {
      const writer = __tachyonTac.service('writer')
      const module = await import('/.tachyon/services/WriterService.js')
      const twin = new module.default({})
      try {
        await twin.inspect()
        const opened = { original: writer.identity, twin: twin.identity, originalContext: writer.contextID, twinContext: twin.contextID, count: twin.count }
        await twin.increment()
        await writer.inspect()
        return { opened, changed: { original: writer.count, twin: twin.count } }
      } finally { twin.__tachyonDispose() }
    })
    diagnostics.observations.control = proof
    assert.match(proof.opened.original, /^[a-f\d-]{36}$/)
    assert.equal(proof.opened.original, proof.opened.twin, 'existing same-service Setup registry shares the configured adapter')
    assert.notEqual(proof.opened.originalContext, proof.opened.twinContext)
    assert.equal(proof.opened.count, 0)
    assert.deepEqual(proof.changed, { original: 1, twin: 1 })
    await page.evaluate(async () => { await __tachyonTac.service('writer').inspect() })
    const retained = await GoPageProviderIdentity.controlState(page)
    assert.equal(retained.identity, proof.opened.original, 'one remaining context retains the same configuration')
    assert.equal(retained.count, 1)
    assert.equal(retained.disposals, 0)
    await GoPageProviderIdentity.waitEvents(diagnostics, 'isolated', 'construct', 1)
    await GoPageProviderIdentity.waitEvents(diagnostics, 'isolated', 'dispose-context', 1)
    return { proof, retained }
  }

  static async controlState(page) {
    return await page.evaluate(() => {
      const writer = __tachyonTac.service('writer')
      return { identity: writer.identity, count: writer.count, constructed: writer.constructed, context: writer.contextID, disposals: writer.contextDisposals }
    })
  }

  static async lifetime(page, diagnostics, identity) {
    diagnostics.phase = 'last-component-and-persisted-page-lifetime'
    await page.locator('#hide').click()
    await page.locator('#writer-panel').waitFor({ state: 'detached' })
    await page.locator('#reader-panel').waitFor({ state: 'detached' })
    const retained = await GoPageProviderIdentity.state(page)
    assert.equal(retained.writer.identity, identity)
    assert.equal(retained.reader.identity, identity)
    assert.equal(GoPageProviderIdentity.events(diagnostics, 'first', 'dispose-context').length, 0, 'page singletons survive removal of the last component')
    await page.evaluate(() => dispatchEvent(new PageTransitionEvent('pagehide', { persisted: true })))
    assert.deepEqual(await GoPageProviderIdentity.state(page), retained)
    assert.equal(GoPageProviderIdentity.events(diagnostics, 'first', 'dispose-context').length, 0, 'a persisted document keeps its providers')
    await page.locator('#show').click()
    await page.locator('#writer-read').waitFor()
    assert.deepEqual(await GoPageProviderIdentity.state(page), retained)
  }

  static async releases(page, diagnostics, identity) {
    diagnostics.phase = 'idempotent-context-release-and-last-release-reconfiguration'
    const proof = await page.evaluate(async () => {
      const writer = __tachyonTac.service('writer')
      const reader = __tachyonTac.service('reader')
      await writer.release()
      await writer.release()
      let rejected
      try { await writer.inspect(); rejected = { code: 'accepted' } } catch (error) { rejected = { code: error.code, message: String(error) } }
      await reader.increment()
      const remaining = { identity: reader.identity, count: reader.count, disposals: reader.contextDisposals }
      await writer.open()
      await writer.inspect()
      const joined = writer.identity
      await writer.release()
      await reader.release()
      await writer.open()
      await reader.open()
      await writer.inspect()
      await reader.inspect()
      return { rejected, remaining, joined, fresh: writer.identity, readerFresh: reader.identity, freshCount: writer.count, writerDisposals: writer.contextDisposals, readerDisposals: reader.contextDisposals }
    })
    diagnostics.observations.released = proof
    assert.ok(proof.rejected.code === 'disposed' || /Behavior context is disposed\./.test(proof.rejected.message), 'released contexts must fail through the real behavior runtime')
    assert.equal(proof.remaining.identity, identity)
    assert.equal(proof.remaining.count, 2)
    assert.equal(proof.remaining.disposals, 0)
    assert.equal(proof.joined, identity, 'an open reader keeps the same provider configuration')
    assert.notEqual(proof.fresh, identity, 'last context release forgets the old configured provider')
    assert.equal(proof.fresh, proof.readerFresh)
    assert.equal(proof.freshCount, 0)
    assert.equal(proof.writerDisposals, 2)
    assert.equal(proof.readerDisposals, 1)
    await GoPageProviderIdentity.waitEvents(diagnostics, 'first', 'construct', 2)
    await GoPageProviderIdentity.waitEvents(diagnostics, 'first', 'dispose-context', 3)
    return proof
  }

  static async pageExit(page, diagnostics, identity) {
    diagnostics.phase = 'page-exit-disposal-and-fresh-provider-control'
    const proof = await page.evaluate(async () => {
      const writer = __tachyonTac.service('writer')
      const reader = __tachyonTac.service('reader')
      dispatchEvent(new PageTransitionEvent('pagehide', { persisted: false }))
      dispatchEvent(new PageTransitionEvent('pagehide', { persisted: false }))
      const stale = await Promise.all([writer, reader].map(async service => {
        try { await service.inspect(); return { code: 'accepted' } } catch (error) { return { code: error.code, message: String(error) } }
      }))
      const module = await import('/.tachyon/services/WriterService.js')
      const fresh = new module.default({})
      try {
        await fresh.inspect()
        return { stale, identity: fresh.identity, count: fresh.count }
      } finally { fresh.__tachyonDispose() }
    })
    diagnostics.observations.exited = proof
    assert.ok(proof.stale.every(error => /disposed|Stale native owner|TY_NATIVE_OWNER/.test(`${error.code} ${error.message}`)))
    assert.notEqual(proof.identity, identity, 'page exit releases the last configured provider')
    assert.equal(proof.count, 0)
    await GoPageProviderIdentity.waitEvents(diagnostics, 'first', 'dispose-context', 6)
    await GoPageProviderIdentity.waitEvents(diagnostics, 'first', 'construct', 3)
    const disposals = GoPageProviderIdentity.events(diagnostics, 'first', 'dispose-context')
    assert.equal(disposals.length, 6, 'three explicit releases, two page-owned releases and one fresh control release')
    assert.equal(new Set(disposals.map(event => event.context)).size, 6, 'each real context is disposed once')
    assert.equal(GoPageProviderIdentity.events(diagnostics, 'first', 'construct').length, 3)
    return { ...proof, contextDisposals: disposals.length, configurationReleaseWitnesses: { lastContextRelease: 1, pageExit: 1, total: 2 } }
  }

  static async probe(engine, origin, output) {
    assert.ok(GoPageProviderIdentity.engines.includes(engine))
    const diagnostics = { engine, phase: 'browser-start', errors: [], failedRequests: [], events: [], observations: {} }
    let browser
    try {
      const playwright = await import('playwright')
      browser = await playwright[engine].launch({ timeout: GoPageProviderIdentity.timeout })
      const context = await browser.newContext()
      const first = await context.newPage()
      const isolated = await context.newPage()
      GoPageProviderIdentity.observe(first, 'first', diagnostics)
      GoPageProviderIdentity.observe(isolated, 'isolated', diagnostics)
      const control = await GoPageProviderIdentity.control(isolated, origin, diagnostics)
      diagnostics.phase = 'shared-provider-identity'
      const initial = await GoPageProviderIdentity.initial(first, origin)
      diagnostics.observations.initial = initial
      GoPageProviderIdentity.shared(initial)
      await GoPageProviderIdentity.waitEvents(diagnostics, 'first', 'construct', 1)
      assert.notEqual(control.retained.identity, initial.writer.identity, 'separate pages retain separate configured providers')
      await first.locator('#writer-increment').click()
      await first.waitForFunction(() => __tachyonTac.service('writer').count === 1, undefined, { timeout: GoPageProviderIdentity.timeout })
      await first.locator('#reader-read').click()
      await first.waitForFunction(() => __tachyonTac.service('reader').count === 1, undefined, { timeout: GoPageProviderIdentity.timeout })
      const changed = await GoPageProviderIdentity.state(first)
      diagnostics.observations.changed = changed
      assert.equal(changed.writer.count, 1)
      assert.equal(changed.reader.count, 1, 'mutation through one Service reaches the other Service')
      await GoPageProviderIdentity.lifetime(first, diagnostics, initial.writer.identity)
      const released = await GoPageProviderIdentity.releases(first, diagnostics, initial.writer.identity)
      assert.deepEqual(await GoPageProviderIdentity.controlState(isolated), control.retained, 'another page survives every first-page release')
      const exited = await GoPageProviderIdentity.pageExit(first, diagnostics, released.fresh)
      assert.deepEqual(await GoPageProviderIdentity.controlState(isolated), control.retained)
      await isolated.evaluate(() => {
        dispatchEvent(new PageTransitionEvent('pagehide', { persisted: false }))
        dispatchEvent(new PageTransitionEvent('pagehide', { persisted: false }))
      })
      await GoPageProviderIdentity.waitEvents(diagnostics, 'isolated', 'dispose-context', 2)
      await GoPageProviderIdentity.waitEvents(diagnostics, 'isolated', 'construct', 1)
      assert.deepEqual(diagnostics.errors, [])
      assert.deepEqual(diagnostics.failedRequests, [])
      diagnostics.phase = 'complete'
      await writeFile(path.join(output, `${engine}-receipt.json`), JSON.stringify({ ...diagnostics, status: 'pass', browserVersion: browser.version(), initial, changed, control, released, exited }, null, 2) + '\n')
    } catch (error) {
      await writeFile(path.join(output, `${engine}-receipt.json`), JSON.stringify({ ...diagnostics, status: 'fail', error: String(error) }, null, 2) + '\n')
      throw error
    } finally { await browser?.close() }
  }

  static async main() {
    const emission = GoPageProviderIdentity.option('--emit-source')
    if (emission) return await GoPageProviderIdentity.emit(path.resolve(emission))
    const engine = GoPageProviderIdentity.option('--probe')
    if (engine) return await GoPageProviderIdentity.probe(engine, GoPageProviderIdentity.option('--origin'), GoPageProviderIdentity.option('--output'))
    await GoPageProviderIdentity.run()
  }
}

await GoPageProviderIdentity.main()
