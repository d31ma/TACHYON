import { SupervisedCommand } from '../supervised-command.mjs'

/** @typedef {(path:string, options?:RequestInit)=>Promise<Response>} ProbeRequest */
/** @typedef {import('../supervised-command.mjs').CommandOptions & {readinessTimeoutMilliseconds?:number}} ProbeOptions */
/** @typedef {import('../supervised-command.mjs').CommandResult} CommandResult */

class ServerProbe {
  static readinessMilliseconds = 30_000
  static requestMilliseconds = 10_000
  static logBytes = 32768

  /** @template T @param {string} binary @param {string[]} args @param {(request:ProbeRequest)=>Promise<T>} probe @param {ProbeOptions} options */
  static async run(binary, args, probe, options) {
    const scope = new ServerProbe(options)
    return await scope.#run(binary, args, probe)
  }

  /** @param {ProbeOptions} options */
  constructor(options) {
    this.options = options
    this.logs = ''
    this.pending = ''
    this.started = Date.now()
    this.readinessMilliseconds = options.readinessTimeoutMilliseconds ?? ServerProbe.readinessMilliseconds
    this.timeoutMilliseconds = options.timeoutMilliseconds ?? SupervisedCommand.defaultTimeoutMilliseconds
    /** @type {Promise<string>} */
    this.ready = new Promise(resolve => { this.resolveReady = resolve })
  }

  /** @param {Buffer} chunk @param {boolean} stdout */
  #output(chunk, stdout) {
    this.logs = (this.logs + chunk).slice(-ServerProbe.logBytes)
    if (!stdout) return
    this.pending = (this.pending + chunk).slice(-ServerProbe.logBytes)
    const match = this.pending.match(/ready at (http:\/\/127\.0\.0\.1:\d+)\//)
    if (match) this.resolveReady(match[1])
  }

  /** @param {CommandResult} result */
  #readinessFailure(result) {
    if (result.error?.message.includes('ENOENT')) return result.error
    return new Error(`Server exited before readiness (${result.status ?? result.signal}): ${this.logs}${result.error ? `; ${result.error.message}` : ''}`)
  }

  /** @param {CommandResult} result */
  #probeFailure(result) {
    if (result.error) return result.error
    return new Error(`Server exited during probe (${result.status ?? result.signal}): ${this.logs}`)
  }

  /** @param {string} origin @returns {ProbeRequest} */
  #request(origin) {
    return (path, options = {}) => fetch(`${origin}${path}`, {
      ...options,
      signal: AbortSignal.timeout(Math.max(1, Math.min(ServerProbe.requestMilliseconds, this.timeoutMilliseconds - (Date.now() - this.started)))),
    })
  }

  /** @template T @param {string} binary @param {string[]} args @param {(request:ProbeRequest)=>Promise<T>} probe @returns {Promise<T>} */
  async #run(binary, args, probe) {
    if (!Number.isSafeInteger(this.readinessMilliseconds) || this.readinessMilliseconds < 1) throw new Error('Server readiness limit must be a positive safe integer')
    const command = await SupervisedCommand.start(binary, args, {
      ...this.options, timeoutMilliseconds: this.timeoutMilliseconds,
      onStdout: chunk => this.#output(chunk, true), onStderr: chunk => this.#output(chunk, false),
    })
    /** @type {ReturnType<typeof setTimeout>|undefined} */
    let timer
    /** @type {Error|undefined} */
    let failure
    try {
      const origin = await Promise.race([
        this.ready,
        command.completion.then(result => { throw this.#readinessFailure(result) }),
        /** @type {Promise<never>} */ (new Promise((_, reject) => { timer = setTimeout(() => reject(new Error(`Server readiness timed out: ${this.logs}`)), this.readinessMilliseconds) })),
      ])
      clearTimeout(timer)
      return await Promise.race([
        Promise.resolve().then(() => probe(this.#request(origin))),
        command.completion.then(result => { throw this.#probeFailure(result) }),
      ])
    } catch (error) {
      failure = /** @type {Error} */ (error)
      throw error
    } finally {
      clearTimeout(timer)
      const exited = command.exited
      const result = await command.stop()
      if (!result.cleanupConfirmed) throw new Error(`${failure?.message ?? ''}; Server family cleanup unconfirmed: ${result.error?.message ?? this.logs}`)
      if (!failure && result.error) throw result.error
      if (!failure && exited) throw this.#probeFailure(result)
    }
  }
}

/** @template T @param {string} binary @param {string[]} args @param {(request:ProbeRequest)=>Promise<T>} probe @param {ProbeOptions} options @returns {Promise<T>} */
export async function withServer(binary, args, probe, options = {}) {
  return await ServerProbe.run(binary, args, probe, options)
}
