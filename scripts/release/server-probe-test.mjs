#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { ProcessControls } from '../supervised-command-test.mjs'
import { withServer } from './server-probe.mjs'

class ServerProbeControls {
  static options = { timeoutMilliseconds: 700, readinessTimeoutMilliseconds: 250, terminationMilliseconds: 100, cleanupMilliseconds: 2000 }

  static async control(mode, probe, expectation) {
    const fixture = ProcessControls.fixture(mode)
    const started = Date.now()
    let timer
    try {
      const promise = Promise.race([
        withServer(process.execPath, fixture.args, probe, ServerProbeControls.options),
        new Promise((_, reject) => { timer = setTimeout(() => reject(new Error('control watchdog: harness did not settle within 3500ms')), 3500) }),
      ])
      await expectation(promise)
      assert.equal(ProcessControls.live(fixture).length, 0, 'the complete server family must be gone')
      assert.ok(Date.now() - started < 3500, 'server scope must settle within its deadline and cleanup bound')
    } finally {
      clearTimeout(timer)
      ProcessControls.evidence(`server ${mode}`, started, fixture)
      ProcessControls.cleanup(fixture)
    }
  }

  static async main() {
    console.log(`retained server fixtures: ${ProcessControls.root}`)
    await ProcessControls.check('healthy request and return value', () => ServerProbeControls.control('server', async request => (await request('/')).text(), async promise => assert.equal(await promise, 'healthy response')))
    await ProcessControls.check('probe exception keeps original error', () => ServerProbeControls.control('server', async () => { throw new Error('probe assertion failed') }, promise => assert.rejects(promise, /probe assertion failed/)))
    await ProcessControls.check('setup failure includes exit and log', () => ServerProbeControls.control('not-ready-exit', async () => true, promise => assert.rejects(promise, /before readiness \(7\):[\s\S]*before readiness/)))
    await ProcessControls.check('returned probe cleans children and grandchildren', () => ServerProbeControls.control('server-family', async () => 42, async promise => assert.equal(await promise, 42)))
    await ProcessControls.check('readiness deadline cleans family', () => ServerProbeControls.control('not-ready', async () => true, promise => assert.rejects(promise, /readiness timed out/)))
    await ProcessControls.check('probe deadline cleans family', () => ServerProbeControls.control('server-family', async () => new Promise(() => {}), promise => assert.rejects(promise, /deadline/)))
    await ProcessControls.check('late probe return cannot pass absolute deadline', () => ServerProbeControls.control('server', async () => {
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 750)
      return 42
    }, promise => assert.rejects(promise, /deadline/)))
    await ProcessControls.check('early successful leader exit cannot pass', () => ServerProbeControls.control('server-exit', async () => new Promise(resolve => setTimeout(() => resolve('too late'), 1000)), promise => assert.rejects(promise, /exited during probe/)))
    await ProcessControls.check('missing binary rejects promptly', async () => {
      await assert.rejects(withServer(`${ProcessControls.root}/missing`, [], async () => true, ServerProbeControls.options), /ENOENT/)
    })
    await ProcessControls.check('unrelated process remains alive', async () => {
      const sentinel = ProcessControls.fixture('sentinel')
      const child = spawn(process.execPath, sentinel.args, { detached: true, stdio: 'ignore' })
      try {
        await ProcessControls.waitForFamily(sentinel, 1)
        await ServerProbeControls.control('server-family', async () => 42, async promise => assert.equal(await promise, 42))
        assert.equal(ProcessControls.live(sentinel).length, 1)
        ProcessControls.evidence('unrelated sentinel remains alive', Date.now(), sentinel)
      } finally {
        ProcessControls.cleanup(sentinel)
        await new Promise(resolve => child.once('close', resolve))
      }
    })
  }
}

await ServerProbeControls.main()
