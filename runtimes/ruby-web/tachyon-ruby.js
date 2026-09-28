export class TachyonRuby {
  static BAD_DESCRIPTOR = 8
  static SEEK_NOT_SUPPORTED = 70
  static CHARACTER_DEVICE = 2
  static REALTIME_CLOCK = 0
  static OUTPUT_LINE_LIMIT = 65_536
  static OUTPUT_CHUNK_BYTES = 4096
  static OUTPUT_TRUNCATED = '[Ruby console line truncated]'
  static STORAGE_ASK = 'tachyon-storage:ask\n'
  static STORAGE_TAKE = 'tachyon-storage:take\n'
  static SPACE = 32

  /**
   * @param {string | URL | WebAssembly.Module} runtime
   * @param {(line: string, descriptor: number) => void} onOutput
   * @param {(payload: string) => void} onEmit
   * @param {(request: string) => string} [onStorage]
   */
  static async start(runtime, onOutput, onEmit, onStorage) {
    const ruby = new TachyonRuby(onOutput, onEmit, onStorage)
    const module = runtime instanceof WebAssembly.Module ? runtime : await WebAssembly.compileStreaming(fetch(runtime))
    const instance = await WebAssembly.instantiate(module, ruby.imports())
    ruby.exports = instance.exports
    ruby.exports._initialize()
    return ruby
  }

  constructor(onOutput, onEmit, onStorage) {
    this.onOutput = onOutput
    this.onEmit = onEmit
    this.onStorage = onStorage
    this.storageAnswer = null
    this.encoder = new TextEncoder()
    this.decoder = new TextDecoder()
    this.outputStreams = new Map()
    this.exports = null
    this.hostError = null
  }

  imports() {
    return {
      wasi_snapshot_preview1: {
        clock_time_get: (clock, precision, target) => this.writeTime(clock, target),
        fd_fdstat_get: (descriptor, target) => this.writeDescriptorStatus(descriptor, target),
        fd_write: (descriptor, pieces, count, written) => this.writeOutput(descriptor, pieces, count, written),
        fd_close: () => 0,
        fd_seek: () => TachyonRuby.SEEK_NOT_SUPPORTED,
        environ_sizes_get: (count, size) => this.writeEmptyEnvironment(count, size),
        environ_get: () => 0,
      },
      env: { emscripten_notify_memory_growth: () => {} },
      tachyon: { tac_host_emit: (pointer, length) => this.receive(pointer, length) },
    }
  }

  receive(pointer, length) {
    const text = this.readText(pointer, length)
    if (text.startsWith(TachyonRuby.STORAGE_ASK)) {
      this.callHost(() => this.answerStorage(pointer, length, this.askStorage(text.slice(TachyonRuby.STORAGE_ASK.length).trim())))
    } else if (text.startsWith(TachyonRuby.STORAGE_TAKE)) {
      this.callHost(() => this.answerStorage(pointer, length, this.storageAnswer ?? this.encoder.encode('{"error":"No storage answer is waiting."}')))
    } else {
      this.callHost(this.onEmit, text)
    }
  }

  askStorage(request) {
    const answer = this.onStorage ? this.onStorage(request) : JSON.stringify({ error: 'Tachyon storage is not on this page.' })
    return this.encoder.encode(answer)
  }

  answerStorage(pointer, length, answer) {
    const room = this.memory().subarray(pointer, pointer + length)
    const fits = answer.length <= length
    const written = fits ? answer : this.encoder.encode(JSON.stringify({ more: answer.length }))
    this.storageAnswer = fits ? null : answer
    room.fill(TachyonRuby.SPACE)
    room.set(written)
  }

  /** @param {string | Uint8Array} source */
  boot(source) {
    if (typeof source === 'string') return this.call('tac_boot', this.encoder.encode(source), 0)
    return this.call('tac_boot', source, 1)
  }

  /** @param {string} request */
  invoke(request) {
    return this.call('tac_invoke', this.encoder.encode(request))
  }

  memoryBytes() {
    return this.exports.memory.buffer.byteLength
  }

  call(name, input, ...extra) {
    const pointer = this.exports.tac_alloc(Math.max(input.length, 1))
    if (pointer === 0) throw new Error('Ruby runtime could not allocate input memory')
    let answer
    try {
      this.memory().set(input, pointer)
      answer = this.takeAnswer(this.exports[name](pointer, input.length, ...extra))
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
      if (end === -1) throw new Error('Ruby runtime returned an unterminated answer')
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
    const milliseconds = clock === TachyonRuby.REALTIME_CLOCK ? Date.now() : performance.now()
    this.view().setBigUint64(target, BigInt(Math.round(milliseconds * 1e6)), true)
    return 0
  }

  writeDescriptorStatus(descriptor, target) {
    if (descriptor > 2) return TachyonRuby.BAD_DESCRIPTOR
    this.memory().fill(0, target, target + 24)
    this.view().setUint8(target, TachyonRuby.CHARACTER_DEVICE)
    return 0
  }

  writeOutput(descriptor, pieces, count, written) {
    if (descriptor !== 1 && descriptor !== 2) return TachyonRuby.BAD_DESCRIPTOR
    let total = 0
    const stream = this.outputStream(descriptor)
    for (let index = 0; index < count; index += 1) {
      const pointer = this.view().getUint32(pieces + index * 8, true)
      const length = this.view().getUint32(pieces + index * 8 + 4, true)
      for (let offset = 0; offset < length; offset += TachyonRuby.OUTPUT_CHUNK_BYTES) {
        const end = Math.min(length, offset + TachyonRuby.OUTPUT_CHUNK_BYTES)
        const bytes = this.memory().subarray(pointer + offset, pointer + end)
        this.appendOutput(stream, stream.decoder.decode(bytes, { stream: true }), descriptor)
      }
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
    const remaining = TachyonRuby.OUTPUT_LINE_LIMIT - stream.pending.length
    if (text.length <= remaining) {
      stream.pending += text
      return
    }
    this.callHost(this.onOutput, stream.pending + text.slice(0, remaining) + TachyonRuby.OUTPUT_TRUNCATED, descriptor)
    stream.pending = ''
    stream.discarding = true
  }

  writeEmptyEnvironment(count, size) {
    this.view().setUint32(count, 0, true)
    this.view().setUint32(size, 0, true)
    return 0
  }
}
