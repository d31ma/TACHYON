import { execFile, spawn } from 'node:child_process'
import { readFileSync } from 'node:fs'

/** @typedef {{pid:number,parent:number,group:number,started:string,state:string}} ProcessIdentity */
/** @typedef {{cwd?:string,env?:NodeJS.ProcessEnv,timeoutMilliseconds?:number,terminationMilliseconds?:number,cleanupMilliseconds?:number,maxBuffer?:number,failOnOutputLimit?:boolean,onStdout?:(chunk:Buffer)=>void,onStderr?:(chunk:Buffer)=>void}} CommandOptions */
/** @typedef {{status:number|null,signal:NodeJS.Signals|null,stdout:string,stderr:string,error?:Error,timedOut:boolean,cleanupConfirmed:boolean}} CommandResult */

export class SupervisedCommand {
  static defaultTimeoutMilliseconds = 20 * 60 * 1000
  static defaultTerminationMilliseconds = 1000
  static defaultCleanupMilliseconds = 10_000
  static defaultMaxBuffer = 4 * 1024 * 1024
  static pollMilliseconds = 50
  static inspectionMilliseconds = 1000
  static inspectionAttempts = 3
  static processTableBytes = 4 * 1024 * 1024
  static windows = process.platform === 'win32'
  static windowsInspectionMilliseconds = 30_000
  static windowsProcessQuery = "Get-CimInstance Win32_Process | Where-Object CreationDate | ForEach-Object { '{0} {1} {2}' -f $_.ProcessId, $_.ParentProcessId, $_.CreationDate.ToFileTimeUtc() }"

  /** @param {CommandOptions} options */
  constructor(options) {
    this.options = options
    this.started = Date.now()
    this.timeoutMilliseconds = options.timeoutMilliseconds ?? SupervisedCommand.defaultTimeoutMilliseconds
    this.terminationMilliseconds = options.terminationMilliseconds ?? SupervisedCommand.defaultTerminationMilliseconds
    this.cleanupMilliseconds = options.cleanupMilliseconds ?? SupervisedCommand.defaultCleanupMilliseconds
    this.maxBuffer = options.maxBuffer ?? SupervisedCommand.defaultMaxBuffer
    /** @type {Map<number, ProcessIdentity>} */
    this.owned = new Map()
    /** @type {Map<number, ProcessIdentity>} */
    this.baseline = new Map()
    this.stdout = Buffer.alloc(0)
    this.stderr = Buffer.alloc(0)
    /** @type {number|null} */
    this.status = null
    /** @type {NodeJS.Signals|null} */
    this.signal = null
    this.closed = false
    this.exited = false
    this.timedOut = false
    /** @type {number|undefined} */
    this.cleanupStarted = undefined
    this.sentTermination = false
    this.sentKill = false
    /** @type {Error|undefined} */
    this.error = undefined
    /** @type {import('node:child_process').ChildProcessByStdio<null, import('node:stream').Readable, import('node:stream').Readable>|undefined} */
    this.child = undefined
    /** @type {(result:CommandResult)=>void} */
    this.resolve = () => {}
    /** @type {Promise<CommandResult>} */
    this.completion = new Promise(resolve => { this.resolve = resolve })
  }

  /** @param {string} binary @param {string[]} args @param {CommandOptions} options */
  static async start(binary, args, options = {}) {
    const command = new SupervisedCommand(options)
    await command.#launch(binary, args)
    return command
  }

  /** @param {string} binary @param {string[]} args @param {CommandOptions} options */
  static async run(binary, args, options = {}) {
    const command = await SupervisedCommand.start(binary, args, options)
    return await command.completion
  }

  async stop() {
    this.#checkDeadline()
    this.#beginCleanup()
    return await this.completion
  }

  #validate() {
    if (!['darwin', 'linux', 'win32'].includes(process.platform)) throw new Error(`Verified process cleanup is unsupported on ${process.platform}; command was not started`)
    for (const value of [this.timeoutMilliseconds, this.terminationMilliseconds, this.cleanupMilliseconds, this.maxBuffer]) {
      if (!Number.isSafeInteger(value) || value < 1) throw new Error('Command limits must be positive safe integers')
    }
  }

  /** @param {string} binary @param {string[]} args */
  async #launch(binary, args) {
    this.#validate()
    this.baseline = await this.#snapshot()
    if (Date.now() - this.started >= this.timeoutMilliseconds) throw new Error('Command family deadline exceeded before launch; command was not started')
    this.child = spawn(binary, args, { cwd: this.options.cwd, env: this.options.env, detached: !SupervisedCommand.windows, windowsHide: true, stdio: ['ignore', 'pipe', 'pipe'] })
    this.child.stdout.on('data', chunk => this.#output('stdout', chunk))
    this.child.stderr.on('data', chunk => this.#output('stderr', chunk))
    this.child.once('error', error => this.#fail(error))
    this.child.once('exit', (status, signal) => {
      this.status = status
      this.signal = signal
      this.exited = true
    })
    this.child.once('close', () => { this.closed = true })
    void this.#watch()
  }

  /** @param {'stdout'|'stderr'} stream @param {Buffer} chunk */
  #output(stream, chunk) {
    if (this.options.failOnOutputLimit && this[stream].length + chunk.length > this.maxBuffer) this.#fail(new Error(`Command output exceeded ${this.maxBuffer} bytes`))
    this[stream] = Buffer.concat([this[stream], chunk]).subarray(-this.maxBuffer)
    try {
      this.#outputListener(stream)?.(chunk)
    } catch (error) {
      this.#fail(/** @type {Error} */ (error))
    }
  }

  /** @param {'stdout'|'stderr'} stream */
  #outputListener(stream) {
    if (stream === 'stdout') return this.options.onStdout
    return this.options.onStderr
  }

  /** @param {Error} error */
  #fail(error) {
    this.error ??= error
    this.#beginCleanup()
  }

  #beginCleanup() {
    this.cleanupStarted ??= Date.now()
  }

  /** @returns {Promise<Map<number, ProcessIdentity>>} */
  async #snapshot() {
    if (SupervisedCommand.windows) {
      const query = ['-NoProfile', '-NonInteractive', '-Command', SupervisedCommand.windowsProcessQuery]
      return this.#parseWindowsSnapshot(await this.#inspect('powershell.exe', query, SupervisedCommand.windowsInspectionMilliseconds))
    }
    let text = ''
    let failure = null
    for (let attempt = 0; attempt < SupervisedCommand.inspectionAttempts && !text.trim(); attempt++) {
      failure = null
      text = await this.#inspect('ps', ['-axo', 'pid=,ppid=,pgid=,lstart=,stat='], SupervisedCommand.inspectionMilliseconds)
        .catch(error => { failure = error; return '' })
    }
    if (failure) throw failure
    return this.#withKernelStartTimes(this.#parseSnapshot(text))
  }

  /** @param {string} program @param {string[]} args @param {number} timeout @returns {Promise<string>} */
  #inspect(program, args, timeout) {
    return new Promise((resolve, reject) => {
      execFile(program, args, {
        encoding: 'utf8', timeout, maxBuffer: SupervisedCommand.processTableBytes, windowsHide: true,
        env: { ...process.env, LC_ALL: 'C' },
      }, (error, stdout) => {
        if (error) reject(new Error(`Process identity inspection failed: ${error.message}`))
        else resolve(stdout)
      })
    })
  }

  /** @param {string} text */
  #parseWindowsSnapshot(text) {
    /** @type {Map<number, ProcessIdentity>} */
    const table = new Map()
    for (const line of text.trim().split(/\r?\n/)) {
      const match = line.trim().match(/^(\d+) (\d+) (\d+)$/)
      if (!match) throw new Error('Process identity inspection returned an unsupported record')
      table.set(Number(match[1]), { pid: Number(match[1]), parent: Number(match[2]), group: Number(match[1]), started: match[3], state: 'R' })
    }
    return table
  }

  /** @param {ProcessIdentity} identity @param {ProcessIdentity} parent */
  #youngerThan(identity, parent) {
    return !SupervisedCommand.windows || BigInt(identity.started) >= BigInt(parent.started)
  }

  /** @param {Map<number, ProcessIdentity>} table */
  #withKernelStartTimes(table) {
    if (process.platform !== 'linux') return table
    for (const identity of [...table.values()]) {
      const started = this.#kernelStartTime(identity.pid)
      if (started === null) table.delete(identity.pid)
      else identity.started = started
    }
    return table
  }

  /** @param {number} pid */
  #kernelStartTime(pid) {
    try {
      const stat = readFileSync(`/proc/${pid}/stat`, 'utf8')
      const fields = stat.slice(stat.lastIndexOf(')') + 2).split(' ')
      return `boot+${fields[19]}`
    } catch {
      return null
    }
  }

  /** @param {string} text */
  #parseSnapshot(text) {
    /** @type {Map<number, ProcessIdentity>} */
    const table = new Map()
    for (const line of text.trim().split('\n')) {
      const match = line.match(/^\s*(\d+)\s+(\d+)\s+(\d+)\s+(\w{3}\s+\w{3}\s+\d+\s+[\d:]+\s+\d+)\s+(\S+)\s*$/)
      if (!match) throw new Error('Process identity inspection returned an unsupported record')
      const identity = { pid: Number(match[1]), parent: Number(match[2]), group: Number(match[3]), started: match[4], state: match[5] }
      table.set(identity.pid, identity)
    }
    return table
  }

  /** @param {ProcessIdentity|undefined} left @param {ProcessIdentity|undefined} right */
  #sameProcess(left, right) {
    return left?.pid === right?.pid && left?.started === right?.started
  }

  /** @param {Map<number, ProcessIdentity>} table */
  #rememberGroup(table) {
    if (!this.child?.pid) return
    for (const identity of table.values()) {
      if (identity.group !== this.child.pid) continue
      if (this.#sameProcess(identity, this.baseline.get(identity.pid))) throw new Error('Process group contains a pre-existing process; cleanup refused')
      this.#remember(identity)
    }
  }

  /** @param {ProcessIdentity} identity */
  #remember(identity) {
    const previous = this.owned.get(identity.pid)
    if (previous && !this.#sameProcess(previous, identity)) throw new Error(`Owned process identity changed for ${identity.pid}; cleanup refused`)
    this.owned.set(identity.pid, identity)
  }

  /** @param {Map<number, ProcessIdentity>} table */
  #rememberDescendants(table) {
    let added = true
    while (added) {
      added = false
      for (const identity of table.values()) {
        if (this.owned.has(identity.pid)) continue
        const parent = this.owned.get(identity.parent)
        if (!parent || !this.#sameProcess(parent, table.get(parent.pid)) || !this.#youngerThan(identity, parent)) continue
        if (this.#sameProcess(identity, this.baseline.get(identity.pid))) throw new Error('Descendant is a pre-existing process; cleanup refused')
        this.#remember(identity)
        added = true
      }
    }
  }

  /** @param {Map<number, ProcessIdentity>} table */
  #live(table) {
    return [...this.owned.values()].filter(identity => this.#sameProcess(identity, table.get(identity.pid)) && !table.get(identity.pid)?.state.startsWith('Z'))
  }

  /** @param {Map<number, ProcessIdentity>} table */
  #remaining(table) {
    return [...this.owned.values()].filter(identity => this.#sameProcess(identity, table.get(identity.pid)))
  }

  #checkDeadline() {
    if (this.cleanupStarted !== undefined || Date.now() - this.started < this.timeoutMilliseconds) return
    this.timedOut = true
    this.#fail(new Error(`Command family deadline exceeded (${this.timeoutMilliseconds}ms)`))
  }

  /** @param {ProcessIdentity[]} live */
  #checkExit(live) {
    if (!this.exited || this.cleanupStarted !== undefined) return
    if (live.some(identity => identity.pid !== this.child?.pid)) this.#fail(new Error('Command leader exited with live owned descendants'))
    if (this.status !== 0) this.#fail(new Error(`Command exited (${this.status ?? this.signal})`))
    this.#beginCleanup()
  }

  #cleanupExpired() {
    return this.cleanupStarted !== undefined && Date.now() - this.cleanupStarted >= this.cleanupMilliseconds
  }

  /** @param {NodeJS.Signals} signal */
  async #signal(signal) {
    const table = await this.#snapshot()
    this.#rememberGroup(table)
    this.#rememberDescendants(table)
    const live = this.#live(table)
    const grouped = live.filter(identity => !SupervisedCommand.windows && table.get(identity.pid)?.group === this.child?.pid)
    if (grouped.length > 0 && this.child?.pid) this.#send(-this.child.pid, signal)
    for (const identity of live) {
      if (grouped.includes(identity)) continue
      const current = await this.#snapshot()
      if (this.#sameProcess(identity, current.get(identity.pid))) this.#send(identity.pid, signal)
    }
  }

  /** @param {number} pid @param {NodeJS.Signals} signal */
  #send(pid, signal) {
    try { process.kill(pid, signal) } catch (error) { if (/** @type {NodeJS.ErrnoException} */ (error).code !== 'ESRCH') throw error }
  }

  /** @param {ProcessIdentity[]} live */
  async #clean(live) {
    if (live.length === 0) return
    if (!this.sentTermination) {
      this.sentTermination = true
      await this.#signal('SIGTERM')
      return
    }
    if (!this.sentKill && this.cleanupStarted !== undefined && Date.now() - this.cleanupStarted >= this.terminationMilliseconds) {
      this.sentKill = true
      await this.#signal('SIGKILL')
    }
  }

  async #watch() {
    try {
      await this.#watchUntilFinished()
    } catch (error) {
      this.#fail(new Error(`Cleanup unconfirmed: ${/** @type {Error} */ (error).message}`))
      this.#finish(false)
    }
  }

  async #watchUntilFinished() {
    while (true) {
      const exitedBeforeSnapshot = this.exited
      const table = await this.#snapshot()
      this.#rememberGroup(table)
      this.#rememberDescendants(table)
      const live = this.#live(table)
      this.#checkDeadline()
      if (!exitedBeforeSnapshot && this.exited) continue
      this.#checkExit(live)
      if (this.cleanupStarted !== undefined) {
        if (this.#remaining(table).length === 0 && this.closed) return this.#finish(true)
        if (this.#cleanupExpired()) return this.#finish(false)
        await this.#clean(live)
      }
      await new Promise(resolve => setTimeout(resolve, SupervisedCommand.pollMilliseconds))
    }
  }

  /** @param {boolean} cleanupConfirmed */
  #finish(cleanupConfirmed) {
    if (!cleanupConfirmed) {
      this.error = new Error(`${this.error?.message ?? 'Command stopped'}; owned family cleanup unconfirmed`)
      this.child?.stdout.destroy()
      this.child?.stderr.destroy()
      this.child?.unref()
    }
    this.resolve({ status: this.status, signal: this.signal, stdout: this.stdout.toString(), stderr: this.stderr.toString(), error: this.error, timedOut: this.timedOut, cleanupConfirmed })
  }
}
