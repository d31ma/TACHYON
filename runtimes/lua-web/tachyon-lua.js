export class TachyonLua {
  static BAD_DESCRIPTOR = 8
  static SEEK_NOT_SUPPORTED = 70
  static NOT_SUPPORTED = 52
  static CHARACTER_DEVICE = 2
  static REALTIME_CLOCK = 0
  static OUTPUT_LINE_LIMIT = 65_536
  static OUTPUT_TRUNCATED = '[Lua console line truncated]'

  /**
   * @param {string | URL | WebAssembly.Module} runtime
   * @param {(line: string, descriptor: number) => void} onOutput
   * @param {(payload: string) => void} onEmit
   * @param {(request: string) => string} [onAsk]
   */
  static async start(runtime, onOutput, onEmit, onAsk) {
    const lua = new TachyonLua(onOutput, onEmit, onAsk)
    const module = runtime instanceof WebAssembly.Module ? runtime : await WebAssembly.compileStreaming(fetch(runtime))
    const instance = await WebAssembly.instantiate(module, lua.imports(module))
    lua.exports = instance.exports
    lua.exports._initialize()
    return lua
  }

  constructor(onOutput, onEmit, onAsk) {
    this.onOutput = onOutput
    this.onEmit = onEmit
    this.onAsk = onAsk
    this.encoder = new TextEncoder()
    this.decoder = new TextDecoder()
    this.outputStreams = new Map()
    this.exports = null
    this.hostError = null
  }

  imports(module) {
    const wasi = {
      clock_time_get: (clock, precision, target) => this.writeTime(clock, target),
      fd_fdstat_get: (descriptor, target) => this.writeDescriptorStatus(descriptor, target),
      fd_write: (descriptor, pieces, count, written) => this.writeOutput(descriptor, pieces, count, written),
      fd_close: () => 0,
      fd_seek: () => TachyonLua.SEEK_NOT_SUPPORTED,
      environ_sizes_get: (count, size) => this.writeEmptyEnvironment(count, size),
      environ_get: () => 0,
      proc_exit: (code) => { throw new Error(`The Lua runtime stopped with exit code ${code}.`) },
    }
    const env = { emscripten_notify_memory_growth: () => {} }
    for (const { module: space, name, kind } of WebAssembly.Module.imports(module)) {
      if (space === 'wasi_snapshot_preview1' && kind === 'function') wasi[name] ??= () => TachyonLua.NOT_SUPPORTED
      if (space === 'env' && kind === 'function') env[name] ??= () => -TachyonLua.NOT_SUPPORTED
    }
    return {
      wasi_snapshot_preview1: wasi,
      env,
      tachyon: {
        tac_host_emit: (pointer, length) => this.callHost(this.onEmit, this.readText(pointer, length)),
        tac_host_ask: (pointer, length) => this.answer(this.readText(pointer, length)),
      },
    }
  }

  answer(request) {
    let text
    try {
      text = this.onAsk ? this.onAsk(request) : JSON.stringify({ error: 'Tachyon host APIs are not on this page.' })
    } catch (error) {
      text = JSON.stringify({ error: error?.message ?? String(error) })
    }
    const bytes = this.encoder.encode(`${text}\0`)
    const pointer = this.exports.tac_alloc(bytes.length)
    if (pointer === 0) return 0
    this.memory().set(bytes, pointer)
    return pointer
  }

  /** @param {string} source */
  boot(source) {
    return this.call('tac_boot', this.encoder.encode(source))
  }

  /** @param {string} request */
  invoke(request) {
    return this.call('tac_invoke', this.encoder.encode(request))
  }

  memoryBytes() {
    return this.exports.memory.buffer.byteLength
  }

  call(name, input) {
    const pointer = this.exports.tac_alloc(Math.max(input.length, 1))
    if (pointer === 0) throw new Error('The Lua runtime could not allocate input memory')
    let answer
    try {
      this.memory().set(input, pointer)
      answer = this.takeAnswer(this.exports[name](pointer, input.length))
    } finally {
      this.exports.tac_free(pointer)
    }
    this.throwHostError()
    return answer
  }

  callHost(callback, ...values) {
    try {
      callback(...values)
    } catch (error) {
      this.hostError ??= error
    }
  }

  throwHostError() {
    const error = this.hostError
    this.hostError = null
    if (error) throw error
  }

  takeAnswer(pointer) {
    if (pointer === 0) return null
    try {
      const memory = this.memory()
      const end = memory.indexOf(0, pointer)
      if (end === -1) throw new Error('The Lua runtime returned an unterminated answer')
      return this.decoder.decode(memory.subarray(pointer, end))
    } finally {
      this.exports.tac_free(pointer)
    }
  }

  memory() {
    return new Uint8Array(this.exports.memory.buffer)
  }

  view() {
    return new DataView(this.exports.memory.buffer)
  }

  readText(pointer, length) {
    return this.decoder.decode(this.memory().subarray(pointer, pointer + length))
  }

  writeTime(clock, target) {
    const milliseconds = clock === TachyonLua.REALTIME_CLOCK ? Date.now() : performance.now()
    this.view().setBigUint64(target, BigInt(Math.round(milliseconds * 1e6)), true)
    return 0
  }

  writeDescriptorStatus(descriptor, target) {
    if (descriptor > 2) return TachyonLua.BAD_DESCRIPTOR
    this.memory().fill(0, target, target + 24)
    this.view().setUint8(target, TachyonLua.CHARACTER_DEVICE)
    return 0
  }

  writeOutput(descriptor, pieces, count, written) {
    if (descriptor !== 1 && descriptor !== 2) return TachyonLua.BAD_DESCRIPTOR
    let total = 0
    const stream = this.outputStream(descriptor)
    for (let index = 0; index < count; index += 1) {
      const pointer = this.view().getUint32(pieces + index * 8, true)
      const length = this.view().getUint32(pieces + index * 8 + 4, true)
      const bytes = this.memory().subarray(pointer, pointer + length)
      this.appendOutput(stream, stream.decoder.decode(bytes, { stream: true }), descriptor)
      total += length
    }
    this.view().setUint32(written, total, true)
    return 0
  }

  outputStream(descriptor) {
    if (!this.outputStreams.has(descriptor)) {
      this.outputStreams.set(descriptor, { decoder: new TextDecoder(), pending: '', discarding: false })
    }
    return this.outputStreams.get(descriptor)
  }

  appendOutput(stream, text, descriptor) {
    let start = 0
    while (start < text.length) {
      const newline = text.indexOf('\n', start)
      const end = newline === -1 ? text.length : newline
      this.appendOutputPart(stream, text.slice(start, end), descriptor)
      if (newline === -1) return
      if (!stream.discarding) this.callHost(this.onOutput, stream.pending, descriptor)
      stream.pending = ''
      stream.discarding = false
      start = newline + 1
    }
  }

  appendOutputPart(stream, text, descriptor) {
    if (stream.discarding) return
    const remaining = TachyonLua.OUTPUT_LINE_LIMIT - stream.pending.length
    if (text.length <= remaining) {
      stream.pending += text
      return
    }
    this.callHost(this.onOutput, stream.pending + text.slice(0, remaining) + TachyonLua.OUTPUT_TRUNCATED, descriptor)
    stream.pending = ''
    stream.discarding = true
  }

  writeEmptyEnvironment(count, size) {
    this.view().setUint32(count, 0, true)
    this.view().setUint32(size, 0, true)
    return 0
  }
}
