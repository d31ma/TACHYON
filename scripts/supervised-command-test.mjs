#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFileSync } from 'node:child_process'
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
import { randomUUID } from 'node:crypto'

export class ProcessControls {
  static root = mkdtempSync(path.join(tmpdir(), 'tachyon-supervision-controls-'))
  static source = `
    const { spawn } = require('node:child_process');
    const { appendFileSync } = require('node:fs');
    const http = require('node:http');
    const [file, marker, mode, role = 'leader'] = process.argv.slice(2);
    appendFileSync(file, JSON.stringify({ pid: process.pid, marker, role, started: Date.now() }) + '\\n');
    setTimeout(() => process.exit(0), 8000);
    if (mode !== 'server' && mode !== 'server-exit') process.on('SIGTERM', () => {});
    function ready() {
      if (process.send) process.send('ready');
      if (role !== 'leader') return;
      if (mode.startsWith('server')) {
        const server = http.createServer((request, response) => { response.end('healthy response'); });
        server.listen(0, '127.0.0.1', () => {
          console.log('ready at http://127.0.0.1:' + server.address().port + '/');
          if (mode === 'server-exit') setTimeout(() => process.exit(0), 200);
        });
        return;
      }
      console.log('family ready');
      if (mode === 'orphan' || mode === 'inherited') process.exit(0);
      if (mode === 'escaped') setTimeout(() => process.exit(0), 150);
    }
    if (mode === 'healthy') { console.log('healthy stdout'); console.error('healthy stderr'); process.exit(0); }
    if (mode === 'healthy-wait') { console.log('healthy stdout'); setTimeout(() => process.exit(0), 200); return; }
    if (mode === 'output-limit') { console.log('x'.repeat(4096)); process.exit(0); }
    if (mode === 'failed') { console.error('fixture failed'); process.exit(7); }
    if (mode === 'not-ready-exit') { console.error('before readiness'); process.exit(7); }
    if (mode === 'sentinel') { setInterval(() => {}, 1000); return; }
    if (mode === 'server' || mode === 'server-exit') { ready(); return; }
    if (role === 'grandchild') { ready(); return; }
    const child = spawn(process.execPath, [__filename, file, marker, mode, role === 'leader' ? 'child' : 'grandchild'], { detached: mode === 'escaped' && role === 'child', stdio: mode === 'inherited' ? ['ignore', 'inherit', 'inherit', 'ipc'] : ['ignore', 'ignore', 'ignore', 'ipc'] });
    child.once('message', ready);
  `

  static fixture(mode) {
    const marker = randomUUID()
    const report = path.join(ProcessControls.root, `${marker}.jsonl`)
    const script = path.join(ProcessControls.root, `${marker}.cjs`)
    writeFileSync(script, ProcessControls.source)
    return { script, marker, report, args: [script, report, marker, mode] }
  }

  static identities(fixture) {
    if (!existsSync(fixture.report)) return []
    return readFileSync(fixture.report, 'utf8').trim().split('\n').filter(Boolean).map(line => JSON.parse(line))
  }

  static live(fixture) {
    return ProcessControls.identities(fixture).filter(identity => ProcessControls.inspect(identity))
  }

  static inspect(identity, includeZombie = false) {
    try {
      const line = execFileSync('ps', ['-p', String(identity.pid), '-o', 'lstart=,stat=,command='], { encoding: 'utf8', timeout: 1000 }).trim()
      const match = line.match(/^(\w{3}\s+\w{3}\s+\d+\s+[\d:]+\s+\d+)\s+(\S+)\s+(.+)$/)
      if (!match || (!includeZombie && match[2].startsWith('Z')) || !match[3].includes(identity.marker)) return undefined
      return match[1]
    } catch {
      return undefined
    }
  }

  static cleanup(fixture) {
    for (const identity of ProcessControls.identities(fixture).reverse()) {
      const started = ProcessControls.inspect(identity)
      if (!started || ProcessControls.inspect(identity) !== started) continue
      try { process.kill(identity.pid, 'SIGKILL') } catch (error) { if (error.code !== 'ESRCH') throw error }
    }
  }

  static async waitForFamily(fixture, count = 3) {
    const deadline = Date.now() + 2000
    while (ProcessControls.identities(fixture).length < count && Date.now() < deadline) await new Promise(resolve => setTimeout(resolve, 20))
    assert.equal(ProcessControls.identities(fixture).length, count, 'fixture identities must be recorded')
  }

  static evidence(name, started, fixture, result) {
    console.log(JSON.stringify({ control: name, milliseconds: Date.now() - started, identities: ProcessControls.identities(fixture), live: ProcessControls.live(fixture), remaining: ProcessControls.identities(fixture).filter(identity => ProcessControls.inspect(identity, true)), result }))
  }

  static async harness(name) {
    const directory = path.join(ProcessControls.root, name)
    mkdirSync(path.join(directory, 'scripts'), { recursive: true })
    const sourceRoot = path.dirname(fileURLToPath(import.meta.url))
    for (const file of ['cargo-artifacts.mjs', 'supervised-command.mjs']) {
      if (existsSync(path.join(sourceRoot, file))) copyFileSync(path.join(sourceRoot, file), path.join(directory, 'scripts', file))
    }
    const file = name === 'BrowserGate' ? 'browser-gate.mjs' : 'native-window-examples-test.mjs'
    const text = readFileSync(path.join(sourceRoot, file), 'utf8')
      .replace(/(?:await )?(?:BrowserGate\.main|NativeWindowExamples\.run)\(\)\s*$/, '')
    const module = path.join(directory, 'scripts', file)
    writeFileSync(module, `${text}\nexport { ${name} }\n`)
    return (await import(pathToFileURL(module).href))[name]
  }

  static async check(name, action) {
    const started = Date.now()
    try {
      await action()
      console.log(`PASS ${name} ${Date.now() - started}ms`)
    } catch (error) {
      console.error(`FAIL ${name}: ${error.stack}`)
      process.exitCode = 1
    }
  }

  static async harnessControls() {
    process.env.TAC_NATIVE_WINDOW_TIMEOUT_MS = '400'
    process.env.TAC_BROWSER_TIMEOUT_MS = '400'
    const native = await ProcessControls.harness('NativeWindowExamples')
    const browser = await ProcessControls.harness('BrowserGate')
    for (const mode of ['healthy', 'failed', 'orphan', 'hang']) {
      await ProcessControls.check(`native ${mode}`, async () => {
        const fixture = ProcessControls.fixture(mode)
        const started = Date.now()
        try {
          if (mode === 'healthy') assert.equal(await native.command(process.execPath, fixture.args, ProcessControls.root), 'healthy stdout\n')
          else await assert.rejects(async () => native.command(process.execPath, fixture.args, ProcessControls.root), /deadline|descendant|fixture failed/)
          assert.equal(ProcessControls.live(fixture).length, 0, 'native family must be gone')
          assert.ok(Date.now() - started < 4000, 'native completion must be bounded')
        } finally {
          ProcessControls.evidence(`native ${mode}`, started, fixture)
          ProcessControls.cleanup(fixture)
        }
      })
      await ProcessControls.check(`browser ${mode}`, async () => {
        const fixture = ProcessControls.fixture(mode)
        const started = Date.now()
        try {
          const script = path.join(ProcessControls.root, `${fixture.marker}-browser.cjs`)
          writeFileSync(script, `process.argv.push(...${JSON.stringify(fixture.args.slice(1))});\n${ProcessControls.source}`)
          const result = await browser.run({ name: mode, file: script }, ProcessControls.root)
          ProcessControls.evidence(`browser ${mode}`, started, fixture, result)
          assert.equal(result.passed, mode === 'healthy')
          assert.equal(ProcessControls.live(fixture).length, 0, 'browser family must be gone')
          assert.ok(Date.now() - started < 4000, 'browser completion must be bounded')
          assert.match(readFileSync(result.log, 'utf8'), mode === 'healthy' ? /healthy stdout[\s\S]*healthy stderr|healthy stderr[\s\S]*healthy stdout/ : /deadline|descendant|fixture failed/)
        } finally { ProcessControls.cleanup(fixture) }
      })
    }
    await ProcessControls.check('browser jobs1 remains serialized', async () => {
      process.argv.push('--jobs=1')
      const fixtures = [ProcessControls.fixture('healthy-wait'), ProcessControls.fixture('healthy-wait')]
      try {
        assert.equal(browser.mostAtOnceSetting(), 1)
        const scripts = fixtures.map((fixture, index) => {
          const script = path.join(ProcessControls.root, `${fixture.marker}-browser.cjs`)
          writeFileSync(script, `process.argv.push(...${JSON.stringify(fixture.args.slice(1))});\n${ProcessControls.source}`)
          return { name: `serial-${index}`, file: script }
        })
        const results = await browser.runAll(scripts, ProcessControls.root)
        assert.equal(results.length, 2)
        assert.ok(results.every(result => result.passed))
        assert.ok(fixtures.every(fixture => ProcessControls.live(fixture).length === 0))
        assert.ok(ProcessControls.identities(fixtures[1])[0].started - ProcessControls.identities(fixtures[0])[0].started >= 200, 'second job must start after the first 200ms fixture exits')
      } finally {
        process.argv.pop()
        fixtures.forEach(fixture => ProcessControls.cleanup(fixture))
      }
    })
  }

  static async runnerControls() {
    const { SupervisedCommand } = await import('./supervised-command.mjs')
    for (const mode of ['healthy', 'failed', 'orphan', 'hang', 'inherited']) {
      await ProcessControls.check(`runner ${mode}`, async () => {
        const fixture = ProcessControls.fixture(mode)
        const started = Date.now()
        try {
          const result = await SupervisedCommand.run(process.execPath, fixture.args, { timeoutMilliseconds: 400, terminationMilliseconds: 100, cleanupMilliseconds: 2000 })
          ProcessControls.evidence(`runner ${mode}`, started, fixture, { ...result, error: result.error?.message })
          assert.equal(result.cleanupConfirmed, true)
          assert.equal(ProcessControls.live(fixture).length, 0)
          assert.ok(Date.now() - started < 3000)
          if (mode === 'healthy') {
            assert.equal(result.error, undefined)
            assert.equal(result.status, 0)
            assert.equal(result.stdout, 'healthy stdout\n')
            assert.equal(result.stderr, 'healthy stderr\n')
          } else if (mode === 'failed') {
            assert.equal(result.status, 7)
            assert.match(result.stderr, /fixture failed/)
          } else assert.match(result.error?.message ?? '', mode === 'hang' ? /deadline/ : /descendant/)
        } finally { ProcessControls.cleanup(fixture) }
      })
    }
    await ProcessControls.check('runner missing binary', async () => {
      const result = await SupervisedCommand.run(path.join(ProcessControls.root, 'absent-binary'), [], { timeoutMilliseconds: 400 })
      assert.match(result.error.message, /ENOENT/)
      assert.equal(result.cleanupConfirmed, true)
    })
    await ProcessControls.check('observed detached grandchild is identity-checked and cleaned', async () => {
      const fixture = ProcessControls.fixture('escaped')
      const started = Date.now()
      try {
        const command = await SupervisedCommand.start(process.execPath, fixture.args, { timeoutMilliseconds: 1000, terminationMilliseconds: 100, cleanupMilliseconds: 2000 })
        await ProcessControls.waitForFamily(fixture)
        const grandchild = ProcessControls.identities(fixture).find(identity => identity.role === 'grandchild')
        assert.equal(Number(execFileSync('ps', ['-p', String(grandchild.pid), '-o', 'pgid='], { encoding: 'utf8', timeout: 1000 }).trim()), grandchild.pid)
        const result = await command.completion
        ProcessControls.evidence('detached grandchild cleanup', started, fixture, { ...result, error: result.error?.message })
        assert.match(result.error.message, /descendant/)
        assert.equal(result.cleanupConfirmed, true)
        assert.equal(ProcessControls.live(fixture).length, 0)
      } finally { ProcessControls.cleanup(fixture) }
    })
    await ProcessControls.check('runner invalid deadline refuses before spawn', async () => {
      const fixture = ProcessControls.fixture('healthy')
      await assert.rejects(SupervisedCommand.run(process.execPath, fixture.args, { timeoutMilliseconds: NaN }), /positive/)
      assert.equal(ProcessControls.identities(fixture).length, 0)
    })
    await ProcessControls.check('bounded output retains native overflow failure', async () => {
      const fixture = ProcessControls.fixture('output-limit')
      try {
        const result = await SupervisedCommand.run(process.execPath, fixture.args, { timeoutMilliseconds: 1000, maxBuffer: 128, failOnOutputLimit: true })
        assert.match(result.error.message, /output exceeded 128 bytes/)
        assert.ok(Buffer.byteLength(result.stdout) <= 128)
        assert.equal(result.cleanupConfirmed, true)
      } finally { ProcessControls.cleanup(fixture) }
    })
    await ProcessControls.check('leader zero cannot mask cleanup deadline failure', async () => {
      const fixture = ProcessControls.fixture('orphan')
      const started = Date.now()
      try {
        const result = await SupervisedCommand.run(process.execPath, fixture.args, { timeoutMilliseconds: 1000, terminationMilliseconds: 1000, cleanupMilliseconds: 80 })
        ProcessControls.evidence('cleanup deadline after leader zero', started, fixture, { ...result, error: result.error?.message })
        assert.equal(result.status, 0)
        assert.equal(result.cleanupConfirmed, false)
        assert.match(result.error.message, /cleanup unconfirmed/)
      } finally { ProcessControls.cleanup(fixture) }
    })
    await ProcessControls.check('browser unknown and empty names refuse without running', async () => {
      for (const selection of ['definitely-unknown', 'runtime,definitely-unknown', ',']) {
        const result = await SupervisedCommand.run(process.execPath, [fileURLToPath(new URL('./browser-gate.mjs', import.meta.url)), `--only=${selection}`, '--jobs=1', `--logs=${ProcessControls.root}`], { timeoutMilliseconds: 1000 })
        assert.equal(result.status, 1)
        assert.match(result.stderr, /nothing was run/)
        assert.doesNotMatch(result.stdout, /PASS|FAIL/)
        assert.equal(result.cleanupConfirmed, true)
      }
    })
    await ProcessControls.check('unconfirmed inspection cannot produce success', async () => {
      const fixture = ProcessControls.fixture('hang')
      const directory = path.join(ProcessControls.root, 'inspection-failure')
      mkdirSync(directory)
      const counter = path.join(directory, 'counter')
      const script = `#!${process.execPath}\nconst fs = require('node:fs');\nconst child = require('node:child_process');\nconst file = ${JSON.stringify(counter)};\nconst count = fs.existsSync(file) ? Number(fs.readFileSync(file, 'utf8')) : 0;\nfs.writeFileSync(file, String(count + 1));\nif (count > 0) process.exit(9);\nconst result = child.spawnSync('/bin/ps', process.argv.slice(2), { stdio: 'inherit', timeout: 1000 });\nprocess.exit(result.status);\n`
      writeFileSync(path.join(directory, 'ps'), script, { mode: 0o755 })
      const previous = process.env.PATH
      try {
        process.env.PATH = `${directory}:${previous}`
        const result = await SupervisedCommand.run(process.execPath, fixture.args, { timeoutMilliseconds: 400 })
        assert.equal(result.cleanupConfirmed, false)
        assert.match(result.error.message, /unconfirmed/)
        console.log(JSON.stringify({ control: 'inspection failure', result: { ...result, error: result.error.message } }))
      } finally {
        process.env.PATH = previous
        await ProcessControls.waitForFamily(fixture)
        ProcessControls.cleanup(fixture)
      }
    })
  }

  static async main() {
    console.log(`retained process fixtures: ${ProcessControls.root}`)
    await ProcessControls.harnessControls()
    if (existsSync(fileURLToPath(new URL('./supervised-command.mjs', import.meta.url)))) await ProcessControls.runnerControls()
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await ProcessControls.main()
