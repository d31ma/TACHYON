import assert from 'node:assert/strict'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { WindowsCommand } from './windows-command.mjs'

export class WindowsGate {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static probe = fileURLToPath(new URL('./page-steps-probe.ps1', import.meta.url))
  static pageOrigin = 'https://tachyon.local'
  static pollMilliseconds = 100
  static quitMilliseconds = 30_000
  static webViewExitMilliseconds = 30_000

  static requireDesktop() {
    assert.equal(process.platform, 'win32', 'This gate runs on Windows, inside the signed-in desktop session (scripts\\windows\\desktop-run.ps1)')
    const onPath = (process.env.PATH ?? '').split(path.delimiter).some(folder => folder && existsSync(path.join(folder, 'cl.exe')))
    assert.ok(onPath && process.env.TAC_WEBVIEW2_SDK, 'MSVC (cl.exe) and TAC_WEBVIEW2_SDK are not set: run this gate through scripts\\windows\\developer-run.ps1')
  }

  /** @param {string} prefix */
  static fixture(prefix) {
    return mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), prefix))
  }

  /** @param {string} root @param {string} name @param {string} text */
  static write(root, name, text) {
    const target = path.join(root, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static async ty() {
    if (process.env.TAC_BIN) return process.env.TAC_BIN
    console.log('==> building ty (cargo build --locked --bin ty)')
    await WindowsCommand.check('cargo', ['build', '--locked', '--bin', 'ty'], {
      cwd: this.repository, timeoutMilliseconds: 3_600_000, onStderr: chunk => process.stderr.write(chunk),
    })
    return CargoArtifacts.path('debug', 'ty.exe')
  }

  /** @param {string} ty @param {string} project @param {number} timeoutMilliseconds */
  static async build(ty, project, timeoutMilliseconds = 600_000) {
    await WindowsCommand.check(ty, ['build', project, '--target', 'windows'], { cwd: this.repository, timeoutMilliseconds })
  }

  /** @param {string} project @param {string} application */
  static executable(project, application) {
    const file = path.join(project, 'dist', 'windows', application, 'bin', `${application}.exe`)
    assert.ok(existsSync(file), `ty build made no ${file}`)
    return file
  }

  /** @param {string} project */
  static hostSource(project) {
    return path.join(project, 'dist', 'windows', 'project', 'tachyon_host.c')
  }

  /** @param {string} project @param {string} application */
  static bundle(project, application) {
    return path.join(project, 'dist', 'windows', application, 'resources', 'WebBundle')
  }

  /** @param {string} executable */
  static webViewData(executable) {
    return `${executable}.WebView2`
  }

  /** @param {string} application */
  static lifecycleLog(application) {
    return path.join(process.env.LOCALAPPDATA ?? '', 'Tachyon', `${application}.jsonl`)
  }

  /** @param {string} file */
  static readLines(file) {
    if (!existsSync(file)) return []
    return readFileSync(file, 'utf8').split(/\r?\n/).filter(Boolean)
  }

  /** @param {string} executable @param {NodeJS.ProcessEnv} env */
  static launch(executable, env = process.env) {
    return WindowsCommand.start(executable, [], { cwd: path.dirname(executable), env, timeoutMilliseconds: 900_000 })
  }

  /** @param {number} pid @param {Array<[string, unknown]>} steps @param {string} folder @param {NodeJS.ProcessEnv} env */
  static startProbe(pid, steps, folder, env = process.env) {
    mkdirSync(folder, { recursive: true })
    const file = path.join(folder, `steps-${pid}-${Date.now()}.json`)
    writeFileSync(file, JSON.stringify(steps))
    const args = ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', this.probe, '-ProcessId', String(pid), '-StepsFile', file]
    return WindowsCommand.start('powershell', args, {
      cwd: this.repository, env: { ...env, TAC_PROBE_HANDSHAKE: folder }, timeoutMilliseconds: 600_000,
      onStdout: chunk => process.stdout.write(chunk), onStderr: chunk => process.stderr.write(chunk),
    })
  }

  /** @param {WindowsCommand} probe */
  static async finishProbe(probe) {
    const result = await probe.completion
    await probe.stop()
    assert.ok(result.status === 0 && !result.error, `The UI Automation probe failed (${result.status})\n${result.error?.message ?? ''}`)
    return result.stdout
  }

  /** @param {number} pid @param {Array<[string, unknown]>} steps @param {string} folder @param {NodeJS.ProcessEnv} env */
  static async probeSteps(pid, steps, folder, env = process.env) {
    return await this.finishProbe(this.startProbe(pid, steps, folder, env))
  }

  /** @param {WindowsCommand} probe @param {string} folder @param {string} name */
  static async waitForRequest(probe, folder, name) {
    const asked = path.join(folder, `${name}.asked`)
    while (!existsSync(asked)) {
      assert.ok(!probe.exited, `The probe ended before it asked for ${name}`)
      await sleep(this.pollMilliseconds)
    }
  }

  /** @param {string} folder @param {string} name */
  static answerRequest(folder, name) {
    writeFileSync(path.join(folder, `${name}.done`), 'done\n')
  }

  /** @param {WindowsCommand} probe @param {string} folder @param {Record<string, () => Promise<void>>} answers */
  static async answerAll(probe, folder, answers) {
    for (const [name, answer] of Object.entries(answers)) {
      await this.waitForRequest(probe, folder, name)
      await answer()
      this.answerRequest(folder, name)
    }
    return await this.finishProbe(probe)
  }

  /** @param {WindowsCommand} app @param {string} executable */
  static async waitForQuit(app, executable) {
    await app.remember().catch(() => app.family)
    assert.ok(await app.waitForExit(this.quitMilliseconds), `The app was still running ${this.quitMilliseconds / 1000} seconds after it was asked to quit`)
    const result = await app.completion
    assert.equal(result.status, 0, `The app quit with exit code ${result.status}`)
    await this.waitForWebViewExit(executable)
  }

  /** @param {string} executable */
  static async waitForWebViewExit(executable) {
    const folder = this.webViewData(executable).toLowerCase()
    await WindowsCommand.waitUntilGone(
      entry => entry.name.toLowerCase() === 'msedgewebview2.exe' && entry.commandLine.toLowerCase().includes(folder),
      this.webViewExitMilliseconds, `WebView2 processes for ${executable}`)
  }

  /** @param {WindowsCommand} app */
  static async memory(app) {
    const family = WindowsCommand.descendants(await WindowsCommand.processes(), app.pid)
    const bytes = [...family.values()].reduce((total, entry) => total + entry.memory, 0)
    return `MEMORY: ${Math.round(bytes / 1024 / 1024)} MiB working set in the app and its ${family.size - 1} WebView2 processes`
  }

  /** @param {string} ty @param {string} project @param {number} port @param {string} readyPath @param {string} origin */
  static async serve(ty, project, port, readyPath, origin) {
    let log = ''
    const yon = WindowsCommand.start(ty, ['serve', project, '--no-watch', '--port', String(port)], {
      cwd: this.repository, timeoutMilliseconds: 900_000,
      onStdout: chunk => { log = (log + chunk).slice(-32768) }, onStderr: chunk => { log = (log + chunk).slice(-32768) },
    })
    const address = `http://127.0.0.1:${port}`
    const request = (route, options = {}) => fetch(`${address}${route}`, { ...options, signal: AbortSignal.timeout(10_000) })
    const deadline = Date.now() + 120_000
    while (Date.now() < deadline) {
      assert.ok(!yon.exited, `ty serve exited before it was ready:\n${log}`)
      if (/ready at http:\/\/127\.0\.0\.1:\d+\//.test(log)) {
        const answer = await request(readyPath, { headers: { origin } }).catch(() => null)
        if (answer?.ok) return { yon, request, address }
      }
      await sleep(500)
    }
    await yon.stop()
    throw new Error(`ty serve did not answer ${readyPath} within 120 seconds:\n${log}`)
  }

  /** @param {string} folder */
  static remove(folder) {
    rmSync(folder, { recursive: true, force: true, maxRetries: 10, retryDelay: 500 })
  }
}
