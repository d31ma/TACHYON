class TacRecorder {
  static key = Symbol.for('tachyon.tac.recorder')
  static tracksKey = Symbol.for('tachyon.tac.media.tracks')
  static grantTag = 'meta[name="tachyon-browser-apis"]'
  static mediaGrants = ['microphone', 'camera', 'screen']
  static label = /^[A-Za-z0-9_.-]{1,64}$/
  static mostFileName = 128
  static unsafeName = /[\\/:*?"<>|\u0000-\u001f]/
  static formats = Object.freeze({
    mp4: { video: ['video/mp4;codecs=avc1,mp4a.40.2', 'video/mp4'], audio: ['audio/mp4;codecs=mp4a.40.2', 'audio/mp4'], extension: '.mp4' },
    webm: { video: ['video/webm;codecs=vp8,opus', 'video/webm'], audio: ['audio/webm;codecs=opus', 'audio/webm'], extension: '.webm' },
  })
  static width = 1280
  static height = 720
  static framesPerSecond = 30
  static sliceMs = 1000
  static progressMs = 5000
  static privateFolder = 'tachyon-recordings'
  static hostApi = 'recorder'
  static hostChunkBytes = 45 * 1024
  static reasons = Object.freeze({
    NotAllowedError: 'denied',
    SecurityError: 'denied',
    AbortError: 'cancelled',
    QuotaExceededError: 'full',
    TypeError: 'invalid',
  })
  static messages = Object.freeze({
    unsupported: 'Recording is not available here.',
    cancelled: 'The save dialog was closed.',
    denied: 'The person or the system did not allow saving the recording.',
    full: 'There is no disk space left for the recording.',
    invalid: 'A recording call has a bad argument.',
    unknown: 'There is no such recording.',
    failed: 'The recording failed.',
  })
  static current = null
  static hooked = false
  static installed = TacRecorder.install()

  static install() {
    const calls = {
      start: (reply, recording, audioTracks, videoTrack, fileName, format) => TacRecorder.start(reply, recording, audioTracks, videoTrack, fileName, format),
      addAudio: (reply, recording, track) => TacRecorder.addAudio(recording, track),
      removeAudio: (reply, recording, track) => TacRecorder.removeAudio(recording, track),
      showVideo: (reply, recording, track) => TacRecorder.showVideo(recording, track),
      pause: (reply, recording) => TacRecorder.pause(reply, recording),
      resume: (reply, recording) => TacRecorder.resume(reply, recording),
      stop: (reply, recording) => TacRecorder.stop(reply, recording),
    }
    Object.defineProperty(globalThis, TacRecorder.key, {
      configurable: true,
      value: Object.freeze({
        supported: () => TacRecorder.supported(),
        grant: () => TacRecorder.grant(),
        calls: Object.freeze(Object.keys(calls)),
        ...calls,
      }),
    })
    globalThis.addEventListener?.('pagehide', () => TacRecorder.current?.recorder?.stop())
    return true
  }

  static supported() {
    return typeof globalThis.MediaRecorder === 'function' && TacRecorder.pickMime(true, '') !== '' && TacRecorder.sinkKind() !== ''
  }

  static grant() {
    const granted = document.querySelector(TacRecorder.grantTag)?.content.split(' ') ?? []
    if (!granted.includes('files')) return 'files'
    return TacRecorder.mediaGrants.find((name) => granted.includes(name)) ?? 'microphone'
  }

  static sinkKind() {
    if (typeof globalThis.__tachyonNativeHostCall === 'function' && globalThis.__tachyonBrowserApiBridge?.includes(TacRecorder.hostApi) === true) return 'host'
    if (typeof globalThis.showSaveFilePicker === 'function') return 'picker'
    if (typeof globalThis.navigator?.storage?.getDirectory === 'function') return 'private'
    return ''
  }

  static pickMime(video, format) {
    if (typeof globalThis.MediaRecorder?.isTypeSupported !== 'function') return ''
    const order = [String(format ?? ''), 'mp4', 'webm'].filter((name) => name in TacRecorder.formats)
    for (const name of order) {
      const found = TacRecorder.formats[name][video ? 'video' : 'audio'].find((type) => MediaRecorder.isTypeSupported(type))
      if (found) return found
    }
    return ''
  }

  static start(reply, recording, audioTracks, videoTrack, fileName, format) {
    const name = TacRecorder.checkedName(recording)
    TacRecorder.ensureIdle()
    const video = String(videoTrack ?? '')
    const mime = TacRecorder.pickMime(video !== '', format)
    TacRecorder.ensureSupported(mime)
    const kind = mime.includes('webm') ? 'webm' : 'mp4'
    const file = TacRecorder.fileName(fileName, kind)
    const entry = { name, reply, mime, kind, file, video, audio: new Map(), bytes: 0, chunks: 0, started: 0, pausedAt: 0, pausedMs: 0, writing: Promise.resolve(), failed: false, sink: null, recorder: null, timers: [] }
    TacRecorder.current = entry
    TacRecorder.ask(() => TacRecorder.openSink(entry))
      .then((sink) => TacRecorder.begin(entry, sink, TacRecorder.trackList(audioTracks)))
      .catch((error) => TacRecorder.fail(entry, 'start', error))
    return null
  }

  static checkedName(recording) {
    const name = String(recording ?? '')
    if (!TacRecorder.label.test(name)) throw new Error('A recording name is 1 to 64 letters, digits, ".", "_" or "-" (invalid).')
    return name
  }

  static ensureIdle() {
    if (TacRecorder.current) throw new Error('A page makes one recording at a time (invalid).')
  }

  static ensureSupported(mime) {
    if (typeof globalThis.MediaRecorder !== 'function' || mime === '' || TacRecorder.sinkKind() === '') throw new Error('Recording is not available here (unsupported).')
  }

  static fileName(fileName, kind) {
    const text = String(fileName ?? '').trim()
    if (text === '' || text.length > TacRecorder.mostFileName || TacRecorder.unsafeName.test(text)) throw new Error('A file name is 1 to 128 characters without / \\ : * ? " < > | (invalid).')
    const extension = TacRecorder.formats[kind].extension
    return text.toLowerCase().endsWith(extension) ? text : text + extension
  }

  static trackList(audioTracks) {
    return String(audioTracks ?? '').split(' ').filter((track) => track !== '')
  }

  static openSink(entry) {
    const kind = TacRecorder.sinkKind()
    if (kind === 'host') return TacRecorder.hostSink(entry)
    if (kind === 'picker') return TacRecorder.pickerSink(entry)
    if (kind === 'private') return TacRecorder.privateSink(entry)
    throw new Error('This app does not save recordings yet (unsupported).')
  }

  static async hostSink(entry) {
    const opened = await TacRecorder.host('open', { recording: entry.name, fileName: entry.file, format: entry.kind })
    entry.file = String(opened?.fileName ?? entry.file)
    return {
      write: (position, data) => TacRecorder.hostWrite(entry.name, position, new Blob([data])),
      close: () => TacRecorder.host('close', { recording: entry.name }),
      saved: null,
    }
  }

  static async hostWrite(recording, position, blob) {
    for (let offset = 0; offset < blob.size; offset += TacRecorder.hostChunkBytes) {
      const data = await TacRecorder.base64(blob.slice(offset, offset + TacRecorder.hostChunkBytes))
      await TacRecorder.host('write', { recording, position: position + offset, data })
    }
  }

  static async base64(blob) {
    let text = ''
    for (const byte of new Uint8Array(await blob.arrayBuffer())) text += String.fromCharCode(byte)
    return btoa(text)
  }

  static async host(call, values) {
    const raw = await globalThis.__tachyonNativeHostCall(`browserApi.${TacRecorder.hostApi}.${call}`, JSON.stringify(values))
    const answer = typeof raw === 'string' ? JSON.parse(raw) : raw
    if (!answer || 'error' in answer) throw new Error(answer?.error ?? 'The native app could not save the recording (failed).')
    return answer.value
  }

  static async pickerSink(entry) {
    const accept = { [entry.mime.split(';')[0]]: [TacRecorder.formats[entry.kind].extension] }
    const handle = await showSaveFilePicker({ suggestedName: entry.file, types: [{ description: 'Recording', accept }] })
    return TacRecorder.writableSink(await handle.createWritable(), null)
  }

  static async privateSink(entry) {
    const root = await navigator.storage.getDirectory()
    await root.removeEntry(TacRecorder.privateFolder, { recursive: true }).catch(() => null)
    const folder = await root.getDirectoryHandle(TacRecorder.privateFolder, { create: true })
    const handle = await folder.getFileHandle(entry.file, { create: true })
    if (typeof handle.createWritable !== 'function') throw new Error('Recording is not available here (unsupported).')
    return TacRecorder.writableSink(await handle.createWritable(), handle)
  }

  static writableSink(writable, saved) {
    return {
      write: (position, data) => writable.write({ type: 'write', position, data }),
      close: () => writable.close(),
      saved,
    }
  }

  static begin(entry, sink, audioTracks) {
    entry.sink = sink
    entry.context = new AudioContext()
    entry.mix = entry.context.createMediaStreamDestination()
    for (const track of audioTracks) TacRecorder.addAudio(entry.name, track)
    const tracks = entry.mix.stream.getAudioTracks()
    if (entry.video !== '') tracks.unshift(TacRecorder.canvasTrack(entry))
    entry.recorder = new MediaRecorder(new MediaStream(tracks), { mimeType: entry.mime })
    entry.recorder.addEventListener('dataavailable', (event) => TacRecorder.piece(entry, event.data))
    entry.recorder.addEventListener('stop', () => TacRecorder.finished(entry))
    entry.recorder.addEventListener('error', (event) => TacRecorder.fail(entry, 'record', event.error))
    TacRecorder.hookTracks()
    entry.started = Date.now()
    entry.recorder.start(TacRecorder.sliceMs)
    entry.timers.push(setInterval(() => TacRecorder.progress(entry), TacRecorder.progressMs))
    entry.reply(['RecordingStarted', { recording: entry.name, format: entry.kind, fileName: entry.file }])
  }

  static canvasTrack(entry) {
    const canvas = document.createElement('canvas')
    canvas.width = TacRecorder.width
    canvas.height = TacRecorder.height
    entry.canvas = canvas.getContext('2d')
    entry.player = document.createElement('video')
    entry.player.muted = true
    entry.player.playsInline = true
    TacRecorder.showVideo(entry.name, entry.video)
    entry.timers.push(setInterval(() => TacRecorder.draw(entry), 1000 / TacRecorder.framesPerSecond))
    return canvas.captureStream(TacRecorder.framesPerSecond).getVideoTracks()[0]
  }

  static draw(entry) {
    const paint = entry.canvas
    paint.fillStyle = '#000'
    paint.fillRect(0, 0, TacRecorder.width, TacRecorder.height)
    const player = entry.player
    if (!player.srcObject || player.videoWidth === 0) return
    const scale = Math.min(TacRecorder.width / player.videoWidth, TacRecorder.height / player.videoHeight)
    const width = player.videoWidth * scale
    const height = player.videoHeight * scale
    paint.drawImage(player, (TacRecorder.width - width) / 2, (TacRecorder.height - height) / 2, width, height)
  }

  static entry(recording) {
    const entry = TacRecorder.current
    if (!entry || entry.name !== String(recording)) throw new Error(`There is no recording named ${recording} (unknown).`)
    return entry
  }

  static mediaTrack(name, kind) {
    const track = globalThis[TacRecorder.tracksKey]?.track(String(name)) ?? null
    if (!track || track.kind !== kind) throw new Error(`There is no ${kind} track named ${name} (unknown).`)
    return track
  }

  static addAudio(recording, track) {
    const entry = TacRecorder.entry(recording)
    const name = String(track)
    if (entry.audio.has(name)) return null
    const source = entry.context.createMediaStreamSource(new MediaStream([TacRecorder.mediaTrack(name, 'audio')]))
    source.connect(entry.mix)
    entry.audio.set(name, source)
    return null
  }

  static removeAudio(recording, track) {
    const entry = TacRecorder.entry(recording)
    entry.audio.get(String(track))?.disconnect()
    entry.audio.delete(String(track))
    return null
  }

  static showVideo(recording, track) {
    const entry = TacRecorder.entry(recording)
    const name = String(track ?? '')
    if (!entry.player) throw new Error('This recording has no video (invalid).')
    entry.player.srcObject = name === '' ? null : new MediaStream([TacRecorder.mediaTrack(name, 'video')])
    entry.video = name
    if (name !== '') entry.player.play()?.catch?.(() => null)
    return null
  }

  static hookTracks() {
    const tracks = globalThis[TacRecorder.tracksKey]
    if (TacRecorder.hooked || !tracks) return
    TacRecorder.hooked = true
    tracks.ended((name) => TacRecorder.trackEnded(name))
  }

  static trackEnded(name) {
    const entry = TacRecorder.current
    if (!entry) return
    entry.audio.get(name)?.disconnect()
    entry.audio.delete(name)
    if (entry.video === name) TacRecorder.showVideo(entry.name, '')
  }

  static pause(reply, recording) {
    const entry = TacRecorder.entry(recording)
    entry.reply = reply
    if (entry.recorder?.state !== 'recording') return null
    entry.recorder.pause()
    entry.pausedAt = Date.now()
    reply(['RecordingPaused', { recording: entry.name }])
    return null
  }

  static resume(reply, recording) {
    const entry = TacRecorder.entry(recording)
    entry.reply = reply
    if (entry.recorder?.state !== 'paused') return null
    entry.recorder.resume()
    entry.pausedMs += Date.now() - entry.pausedAt
    entry.pausedAt = 0
    reply(['RecordingResumed', { recording: entry.name }])
    return null
  }

  static stop(reply, recording) {
    const entry = TacRecorder.entry(recording)
    entry.reply = reply
    if (entry.recorder?.state === 'inactive' || !entry.recorder) throw new Error(`The recording ${entry.name} has not started yet (invalid).`)
    if (entry.pausedAt !== 0) TacRecorder.resume(() => null, entry.name)
    entry.recorder.stop()
    return null
  }

  static seconds(entry) {
    const paused = entry.pausedAt === 0 ? 0 : Date.now() - entry.pausedAt
    return Math.max(0, Math.round((Date.now() - entry.started - entry.pausedMs - paused) / 100) / 10)
  }

  static progress(entry) {
    if (entry.failed) return
    entry.reply(['RecordingProgress', { recording: entry.name, bytes: entry.bytes, seconds: TacRecorder.seconds(entry) }])
  }

  static piece(entry, blob) {
    if (entry.failed || blob.size === 0) return
    const first = entry.chunks === 0
    entry.chunks += 1
    entry.writing = entry.writing
      .then(() => (first && entry.kind === 'webm' ? TacRecorder.withDuration(blob) : blob))
      .then((data) => TacRecorder.written(entry, data))
      .catch((error) => TacRecorder.fail(entry, 'record', error))
  }

  static async written(entry, data) {
    const bytes = data instanceof Blob ? data : new Blob([data.bytes])
    if (data.durationAt !== undefined) entry.durationAt = entry.bytes + data.durationAt
    await entry.sink.write(entry.bytes, bytes)
    entry.bytes += bytes.size
  }

  static async withDuration(blob) {
    const bytes = new Uint8Array(await blob.arrayBuffer())
    return TacRecorder.webmReserve(bytes) ?? blob
  }

  static finished(entry) {
    if (entry.failed) return
    entry.writing = entry.writing
      .then(() => TacRecorder.patchDuration(entry))
      .then(() => entry.sink.close())
      .then(() => TacRecorder.download(entry))
      .then(() => TacRecorder.done(entry))
      .catch((error) => TacRecorder.fail(entry, 'stop', error))
  }

  static patchDuration(entry) {
    if (entry.durationAt === undefined) return null
    return entry.sink.write(entry.durationAt, TacRecorder.webmValue(TacRecorder.seconds(entry) * 1000))
  }

  static async download(entry) {
    if (!entry.sink.saved) return
    const url = URL.createObjectURL(await entry.sink.saved.getFile())
    const link = document.createElement('a')
    link.href = url
    link.download = entry.file
    link.click()
    setTimeout(() => URL.revokeObjectURL(url), 60000)
  }

  static done(entry) {
    if (entry.failed) return
    TacRecorder.cleanUp(entry)
    entry.reply(['RecordingStopped', { recording: entry.name, bytes: entry.bytes, seconds: TacRecorder.seconds(entry) }])
  }

  static fail(entry, call, error) {
    if (entry.failed) return
    entry.failed = true
    const reason = TacRecorder.reasons[error?.name] ?? TacRecorder.reasonIn(error)
    TacRecorder.cleanUp(entry)
    if (entry.recorder?.state !== 'inactive') entry.recorder?.stop()
    entry.sink?.close().catch(() => null)
    entry.reply(['RecordingFailed', { recording: entry.name, call, reason, message: TacRecorder.messages[reason] ?? TacRecorder.messages.failed }])
  }

  static reasonIn(error) {
    const word = /\((unsupported|unknown|invalid|cancelled|denied|full)\)/.exec(String(error?.message ?? ''))
    return word ? word[1] : 'failed'
  }

  static cleanUp(entry) {
    for (const timer of entry.timers) clearInterval(timer)
    for (const source of entry.audio.values()) source.disconnect()
    entry.audio.clear()
    if (entry.player) entry.player.srcObject = null
    entry.context?.close().catch(() => null)
    if (TacRecorder.current === entry) TacRecorder.current = null
  }

  static ask(work) {
    return new Promise((resolve) => resolve(work()))
  }

  static webmSegment = [0x18, 0x53, 0x80, 0x67]
  static webmInfo = [0x15, 0x49, 0xa9, 0x66]
  static webmDurationId = [0x44, 0x89, 0x88]
  static webmDurationBytes = 11

  static webmReserve(bytes) {
    const segment = TacRecorder.webmFind(bytes, TacRecorder.webmSegment, 0)
    if (segment < 0) return null
    const segmentSize = TacRecorder.webmVint(bytes, segment + 4)
    const info = TacRecorder.webmFind(bytes, TacRecorder.webmInfo, segment + 4 + segmentSize.length)
    if (info < 0 || !segmentSize.unknown) return null
    const size = TacRecorder.webmVint(bytes, info + 4)
    const bodyStart = info + 4 + size.length
    const bodyEnd = bodyStart + size.value
    if (bodyEnd > bytes.length) return null
    const existing = TacRecorder.webmFind(bytes.subarray(bodyStart, bodyEnd), TacRecorder.webmDurationId, 0)
    if (existing >= 0) return { bytes, durationAt: bodyStart + existing + TacRecorder.webmDurationId.length }
    const header = TacRecorder.webmSizeBytes(size.value + TacRecorder.webmDurationBytes, size.length)
    const element = new Uint8Array([...TacRecorder.webmDurationId, 0, 0, 0, 0, 0, 0, 0, 0])
    const out = new Uint8Array(bytes.length + TacRecorder.webmDurationBytes + header.length - size.length)
    out.set(bytes.subarray(0, info + 4))
    out.set(header, info + 4)
    const body = info + 4 + header.length
    out.set(bytes.subarray(bodyStart, bodyEnd), body)
    const durationAt = body + size.value
    out.set(element, durationAt)
    out.set(bytes.subarray(bodyEnd), durationAt + element.length)
    return { bytes: out, durationAt: durationAt + TacRecorder.webmDurationId.length }
  }

  static webmValue(milliseconds) {
    const bytes = new Uint8Array(8)
    new DataView(bytes.buffer).setFloat64(0, milliseconds)
    return bytes
  }

  static webmFind(bytes, pattern, from) {
    for (let index = from; index <= bytes.length - pattern.length; index += 1) {
      if (pattern.every((value, offset) => bytes[index + offset] === value)) return index
    }
    return -1
  }

  static webmVint(bytes, at) {
    const first = bytes[at]
    let length = 1
    while (length <= 8 && (first & (0x80 >> (length - 1))) === 0) length += 1
    let value = first & (0xff >> length)
    let unknown = value === (0xff >> length)
    for (let index = 1; index < length; index += 1) {
      value = value * 256 + bytes[at + index]
      unknown = unknown && bytes[at + index] === 0xff
    }
    return { value, length, unknown }
  }

  static webmSizeBytes(value, length) {
    const width = Math.max(length, TacRecorder.webmWidthFor(value))
    const out = new Uint8Array(width)
    let rest = value
    for (let index = width - 1; index >= 0; index -= 1) {
      out[index] = rest % 256
      rest = Math.floor(rest / 256)
    }
    out[0] |= 0x80 >> (width - 1)
    return out
  }

  static webmWidthFor(value) {
    let width = 1
    while (value >= 2 ** (7 * width) - 1) width += 1
    return width
  }
}
