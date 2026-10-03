#!/usr/bin/env node
import { execFileSync, spawn } from 'node:child_process'
import { createWriteStream, existsSync, mkdirSync } from 'node:fs'
import { cpus, freemem, tmpdir, totalmem } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

/** @typedef {{name:string,file:string}} BrowserScript */
/** @typedef {{name:string,passed:boolean,seconds:number,log:string}} ScriptResult */
/** @typedef {{found:BrowserScript[],missing:string[]}} ScriptChoice */

class BrowserGate {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..')
  static names = ['website', 'runtime', 'storage', 'cached-client', 'web-apis', 'tac-client', 'behavior', 'events', 'web-components']
  static mostAtOnce = 4
  static leastFreePercent = 25
  static freePercentPerScript = 12
  static waitMilliseconds = 2000
  static millisecondsPerSecond = 1000
  static macFreeLine = /free percentage:\s*(\d+)%/

  /** @returns {ScriptChoice} */
  static scripts() {
    const chosen = BrowserGate.option('--only=')?.split(',').filter(name => name !== '') ?? BrowserGate.names
    /** @type {ScriptChoice} */
    const choice = { found: [], missing: [] }
    for (const name of chosen) {
      const candidates = [`scripts/${name}-browser-test.mjs`, `scripts/${name}-test.mjs`]
      const file = candidates.find(candidate => existsSync(path.join(BrowserGate.repo, candidate)))
      if (file) choice.found.push({ name, file })
      else choice.missing.push(name)
    }
    return choice
  }

  /**
   * @param {string} prefix
   * @returns {string|undefined}
   */
  static option(prefix) {
    return process.argv.find(argument => argument.startsWith(prefix))?.slice(prefix.length)
  }

  /** @returns {number} */
  static mostAtOnceSetting() {
    const configured = Number(BrowserGate.option('--jobs=') ?? process.env.TAC_BROWSER_JOBS)
    if (Number.isFinite(configured) && configured >= 1) return Math.floor(configured)
    const byCores = Math.max(1, Math.floor(cpus().length / 3))
    return Math.min(BrowserGate.mostAtOnce, byCores)
  }

  /** @returns {number} */
  static freeMemoryPercent() {
    if (process.platform !== 'darwin') return Math.round((freemem() / totalmem()) * 100)
    try {
      const report = execFileSync('memory_pressure', [], { encoding: 'utf8' })
      return Number(report.match(BrowserGate.macFreeLine)?.[1] ?? 100)
    } catch {
      return 100
    }
  }

  /**
   * @param {number} running
   * @returns {boolean}
   */
  static hasRoomForAnother(running) {
    if (running === 0) return true
    const needed = BrowserGate.leastFreePercent + BrowserGate.freePercentPerScript
    return BrowserGate.freeMemoryPercent() >= needed
  }

  /**
   * @param {BrowserScript} script
   * @param {string} logFolder
   * @returns {Promise<ScriptResult>}
   */
  static run(script, logFolder) {
    const log = path.join(logFolder, `${script.name}.txt`)
    const output = createWriteStream(log)
    const started = Date.now()
    return new Promise(resolve => {
      const child = spawn(process.execPath, [script.file], { cwd: BrowserGate.repo, stdio: ['ignore', 'pipe', 'pipe'] })
      child.stdout.pipe(output, { end: false })
      child.stderr.pipe(output, { end: false })
      child.once('close', code => {
        output.end()
        const seconds = Math.round((Date.now() - started) / BrowserGate.millisecondsPerSecond)
        resolve({ name: script.name, passed: code === 0, seconds, log })
      })
    })
  }

  /**
   * @param {BrowserScript[]} scripts
   * @param {string} logFolder
   * @returns {Promise<ScriptResult[]>}
   */
  static async runAll(scripts, logFolder) {
    const limit = BrowserGate.mostAtOnceSetting()
    const waiting = [...scripts]
    /** @type {Set<Promise<ScriptResult>>} */
    const running = new Set()
    /** @type {ScriptResult[]} */
    const results = []
    while (waiting.length > 0 || running.size > 0) {
      const canStart = waiting.length > 0 && running.size < limit && BrowserGate.hasRoomForAnother(running.size)
      if (canStart) {
        const script = /** @type {BrowserScript} */ (waiting.shift())
        const job = BrowserGate.run(script, logFolder).then(result => {
          running.delete(job)
          results.push(result)
          console.log(`${result.passed ? 'PASS' : 'FAIL'} ${result.name} ${result.seconds}s (${result.log})`)
          return result
        })
        running.add(job)
        continue
      }
      const pause = new Promise(resolve => setTimeout(resolve, BrowserGate.waitMilliseconds))
      await Promise.race([...running, pause])
    }
    return results
  }

  static async main() {
    const logFolder = BrowserGate.option('--logs=') ?? path.join(tmpdir(), 'tachyon-browser-gate')
    mkdirSync(logFolder, { recursive: true })
    const { found, missing } = BrowserGate.scripts()
    if (missing.length > 0 || found.length === 0) {
      console.error(`browser tests: no script for ${missing.join(' ') || 'any name'}; nothing was run`)
      process.exitCode = 1
      return
    }
    const started = Date.now()
    const results = await BrowserGate.runAll(found, logFolder)
    const failed = results.filter(result => !result.passed).map(result => result.name)
    const wall = Math.round((Date.now() - started) / BrowserGate.millisecondsPerSecond)
    const sum = results.reduce((total, result) => total + result.seconds, 0)
    console.log(`browser tests: ${results.length - failed.length} passed, ${failed.length} failed (${failed.join(' ') || 'none'}); ${wall}s wall, ${sum}s summed, at most ${BrowserGate.mostAtOnceSetting()} at once`)
    process.exitCode = failed.length > 0 || results.length === 0 ? 1 : 0
  }
}

await BrowserGate.main()
