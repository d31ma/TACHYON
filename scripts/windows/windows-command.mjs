import { execFile, spawn } from 'node:child_process'
import { setTimeout as sleep } from 'node:timers/promises'

/** @typedef {{pid:number,parent:number,name:string,started:string,commandLine:string,memory:number}} WindowsProcess */
/** @typedef {{cwd?:string,env?:NodeJS.ProcessEnv,timeoutMilliseconds?:number,onStdout?:(chunk:Buffer)=>void,onStderr?:(chunk:Buffer)=>void}} WindowsCommandOptions */
/** @typedef {{status:number|null,stdout:string,stderr:string,timedOut:boolean,error?:Error}} WindowsCommandResult */

export class WindowsCommand {
  static defaultTimeoutMilliseconds = 20 * 60 * 1000
  static cleanupMilliseconds = 20_000
  static pollMilliseconds = 250
  static maxBuffer = 4 * 1024 * 1024
  static processListing = 'Get-CimInstance Win32_Process | ForEach-Object { [pscustomobject]@{ pid = [int]$_.ProcessId; parent = [int]$_.ParentProcessId; name = [string]$_.Name; started = $(if ($_.CreationDate) { [string]$_.CreationDate.ToFileTimeUtc() } else { "0" }); commandLine = [string]$_.CommandLine; memory = [double]$_.WorkingSetSize } } | ConvertTo-Json -Compress'

  /** @param {string} binary @param {string[]} args @param {WindowsCommandOptions} options */
  constructor(binary, args, options) {
    if (process.platform !== 'win32') throw new Error('WindowsCommand runs only on Windows')
    this.label = `${binary} ${args.join(' ')}`
    this.options = options
    this.stdout = ''
    this.stderr = ''
    this.status = null
    this.exited = false
    this.timedOut = false
    /** @type {Error|undefined} */
    this.error = undefined
    /** @type {Map<number, WindowsProcess>} */
    this.family = new Map()
    this.child = spawn(binary, args, { cwd: options.cwd, env: options.env, stdio: ['ignore', 'pipe', 'pipe'], windowsHide: false })
    this.pid = this.child.pid ?? 0
    this.child.stdout.on('data', chunk => this.#output('stdout', chunk))
    this.child.stderr.on('data', chunk => this.#output('stderr', chunk))
    this.child.once('error', error => { this.error ??= error })
    /** @type {Promise<WindowsCommandResult>} */
    this.completion = new Promise(resolve => {
      this.child.once('close', status => {
        this.status = status
        this.exited = true
        clearTimeout(this.timer)
        resolve({ status, stdout: this.stdout, stderr: this.stderr, timedOut: this.timedOut, error: this.error })
      })
    })
    this.timer = setTimeout(() => {
      this.timedOut = true
      this.error ??= new Error(`${this.label} ran longer than ${this.timeout()} ms`)
      this.stop().catch(error => { this.error = new Error(`${this.error?.message}; ${error.message}`) })
    }, this.timeout())
  }

  /** @param {string} binary @param {string[]} args @param {WindowsCommandOptions} options */
  static start(binary, args, options = {}) {
    return new WindowsCommand(binary, args, options)
  }

  /** @param {string} binary @param {string[]} args @param {WindowsCommandOptions} options */
  static async run(binary, args, options = {}) {
    const command = WindowsCommand.start(binary, args, options)
    const result = await command.completion
    await command.stop()
    return result
  }

  /** @param {string} binary @param {string[]} args @param {WindowsCommandOptions} options */
  static async check(binary, args, options = {}) {
    const result = await WindowsCommand.run(binary, args, options)
    if (result.status !== 0 || result.error) {
      throw new Error(`${binary} ${args.join(' ')} failed (${result.status})\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    }
    return result.stdout
  }

  /** @returns {Promise<WindowsProcess[]>} */
  static processes() {
    return new Promise((resolve, reject) => {
      execFile('powershell', ['-NoProfile', '-NonInteractive', '-Command', WindowsCommand.processListing], {
        encoding: 'utf8', timeout: 30_000, maxBuffer: 64 * 1024 * 1024, windowsHide: true,
      }, (error, stdout) => {
        if (error) reject(new Error(`Listing the Windows processes failed: ${error.message}`))
        else resolve([JSON.parse(stdout)].flat())
      })
    })
  }

  /** @param {WindowsProcess[]} table @param {number} root */
  static descendants(table, root) {
    const found = new Map()
    const rootProcess = table.find(entry => entry.pid === root)
    if (rootProcess) found.set(root, rootProcess)
    let added = true
    while (added) {
      added = false
      for (const entry of table) {
        if (found.has(entry.pid) || !found.has(entry.parent)) continue
        if (Number(entry.started) < Number(found.get(entry.parent).started)) continue
        found.set(entry.pid, entry)
        added = true
      }
    }
    return found
  }

  /** @param {(entry: WindowsProcess) => boolean} matches @param {number} milliseconds @param {string} label */
  static async waitUntilGone(matches, milliseconds, label) {
    const deadline = Date.now() + milliseconds
    let left = []
    do {
      left = (await WindowsCommand.processes()).filter(matches)
      if (left.length === 0) return
      await sleep(WindowsCommand.pollMilliseconds * 4)
    } while (Date.now() < deadline)
    throw new Error(`${label} still running after ${milliseconds} ms: ${left.map(entry => `${entry.name} ${entry.pid}`).join(', ')}`)
  }

  timeout() {
    return this.options.timeoutMilliseconds ?? WindowsCommand.defaultTimeoutMilliseconds
  }

  /** @param {'stdout'|'stderr'} stream @param {Buffer} chunk */
  #output(stream, chunk) {
    this[stream] = (this[stream] + chunk.toString()).slice(-WindowsCommand.maxBuffer)
    const listener = stream === 'stdout' ? this.options.onStdout : this.options.onStderr
    listener?.(chunk)
  }

  async remember() {
    if (this.exited || this.pid === 0) return this.family
    for (const [pid, entry] of WindowsCommand.descendants(await WindowsCommand.processes(), this.pid)) this.family.set(pid, entry)
    return this.family
  }

  /** @param {number} milliseconds */
  async waitForExit(milliseconds) {
    const deadline = Date.now() + milliseconds
    while (!this.exited && Date.now() < deadline) await sleep(WindowsCommand.pollMilliseconds)
    return this.exited
  }

  async stop() {
    await this.remember().catch(() => this.family)
    if (!this.exited && this.pid !== 0) {
      await new Promise(resolve => execFile('taskkill', ['/PID', String(this.pid), '/T', '/F'], { windowsHide: true }, () => resolve(undefined)))
    }
    if (!(await this.waitForExit(WindowsCommand.cleanupMilliseconds))) throw new Error(`${this.label} did not exit after taskkill`)
    const owned = new Set([...this.family.values()].map(entry => `${entry.pid}:${entry.started}`))
    for (const entry of await WindowsCommand.processes()) {
      if (!owned.has(`${entry.pid}:${entry.started}`)) continue
      await new Promise(resolve => execFile('taskkill', ['/PID', String(entry.pid), '/F'], { windowsHide: true }, () => resolve(undefined)))
    }
    await WindowsCommand.waitUntilGone(entry => owned.has(`${entry.pid}:${entry.started}`), WindowsCommand.cleanupMilliseconds, `processes started by ${this.label}`)
    return await this.completion
  }
}
