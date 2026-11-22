class TacMedia {
  static key = Symbol.for('tachyon.tac.media')
  static tracksKey = Symbol.for('tachyon.tac.media.tracks')
  static label = /^[A-Za-z0-9_.-]{1,64}$/
  static mostTracks = 8
  static deviceKinds = Object.freeze({ audioinput: 'microphone', videoinput: 'camera', audiooutput: 'speaker' })
  static qualities = Object.freeze({
    low: { width: { ideal: 640 }, height: { ideal: 360 } },
    medium: { width: { ideal: 1280 }, height: { ideal: 720 } },
    high: { width: { ideal: 1920 }, height: { ideal: 1080 } },
  })
  static reasons = Object.freeze({
    NotAllowedError: 'denied',
    SecurityError: 'denied',
    NotFoundError: 'notFound',
    OverconstrainedError: 'notFound',
    NotReadableError: 'busy',
    AbortError: 'busy',
    TypeError: 'unsupported',
  })
  static messages = Object.freeze({
    unsupported: 'Capturing audio or video is not available here.',
    denied: 'The person or the system did not allow this device.',
    notFound: 'No such device is connected.',
    busy: 'The device is used by another application.',
    unknown: 'There is no such track or device.',
    failed: 'The media call failed.',
    exhausted: 'The send frame key has used every counter value; send a new key.',
  })
  static grants = Object.freeze({ captureMicrophone: 'microphone', captureCamera: 'camera', captureScreen: 'screen' })
  static grantTag = 'meta[name="tachyon-browser-apis"]'
  static features = Object.freeze({
    capture: ['mediaDevices', 'getUserMedia'],
    screen: ['mediaDevices', 'getDisplayMedia'],
    send: ['RTCRtpSender', 'replaceTrack'],
    simulcast: ['RTCRtpTransceiver', 'setCodecPreferences'],
    frames: ['RTCRtpScriptTransform', ''],
    speakers: ['HTMLMediaElement', 'setSinkId'],
  })
  static framesPath = '/.tachyon/tac-media-frames.js'
  static cryptoHooksKey = Symbol.for('tachyon.tac.crypto.hooks')
  static peersHooksKey = Symbol.for('tachyon.tac.peers.hooks')
  static largestKeyId = Number.MAX_SAFE_INTEGER
  static reportGapMs = 5000
  static vp8 = 'video/vp8'
  static frames = null
  static frameReply = null
  static binding = new Map()
  static encrypted = new Map()
  static watched = new Set()
  static peersHooked = false
  static replies = new Map()
  static lastReply = null
  static mostRemote = 128
  static layers = Object.freeze([
    { rid: 'q', scaleResolutionDownBy: 4, maxBitrate: 150000 },
    { rid: 'h', scaleResolutionDownBy: 2, maxBitrate: 500000 },
    { rid: 'f', scaleResolutionDownBy: 1, maxBitrate: 1500000 },
  ])
  static notReceiving = ['sendonly', 'inactive', 'stopped']
  static sending = new Map()
  static remote = new Map()
  static streams = new Map()
  static blocked = new Set()
  static previousStats = new Map()
  static levelMs = 250
  static speakingOn = 0.05
  static speakingOff = 0.02
  static speakingTicks = 2
  static quietTicks = 4
  static levels = new Map()
  static analysers = new Map()
  static audioContext = null
  static levelTimer = 0
  static speakReply = null
  static loudest = ''
  static loudestCandidate = ''
  static loudestTicks = 0
  static reported = new Map()
  static rejected = new Map()
  static hostApi = 'media'
  static callMessages = Object.freeze({
    'useFrameKey unsupported': 'Frame keys need the crypto browser API.',
    'useFrameKey unknown': 'There is no Crypto key with that name.',
    'callActive failed': 'The call audio could not start or stop.',
  })
  static tracks = new Map()
  static listeners = new Set()
  static changedHooks = []
  static endedHooks = []
  static speaker = ''
  static installed = TacMedia.install()

  static install() {
    const calls = {
      devices: (reply, request) => TacMedia.devices(reply, request),
      chooseSpeaker: (reply, device) => TacMedia.chooseSpeaker(reply, device),
      captureMicrophone: (reply, track, device, processing) => TacMedia.captureMicrophone(reply, track, device, processing),
      captureCamera: (reply, track, device, quality) => TacMedia.captureCamera(reply, track, device, quality),
      captureScreen: (reply, track, withAudio) => TacMedia.captureScreen(reply, track, withAudio),
      stopTrack: (reply, track) => TacMedia.stopTrack(reply, track),
      mute: (reply, track, muted) => TacMedia.mute(track, muted),
      send: (reply, connection, track, stream, simulcast) => TacMedia.send(reply, connection, track, stream, simulcast),
      replace: (reply, connection, track, withTrack) => TacMedia.replace(reply, connection, track, withTrack),
      unsend: (reply, connection, track) => TacMedia.unsend(reply, connection, track),
      stats: (reply, request, connection) => TacMedia.stats(reply, request, connection),
      speaking: (reply, on) => TacMedia.speaking(reply, on),
      resumeAudio: () => TacMedia.resumeAudio(),
      useFrameKey: (reply, keyId, key) => TacMedia.useFrameKey(reply, keyId, key),
      sendFrameKey: (reply, keyId) => TacMedia.postKey(reply, 'send', keyId),
      forgetFrameKey: (reply, keyId) => TacMedia.postKey(reply, 'forget', keyId),
      encryptFrames: (reply, connection, on) => TacMedia.encryptFrames(reply, connection, on),
      callActive: (reply, on, title) => TacMedia.callActive(reply, on, title),
    }
    Object.defineProperty(globalThis, TacMedia.key, {
      configurable: true,
      value: Object.freeze({
        supported: (feature) => TacMedia.supported(feature),
        grant: (call) => TacMedia.grant(call),
        calls: Object.freeze(Object.keys(calls)),
        ...TacMedia.tracked(calls),
      }),
    })
    Object.defineProperty(globalThis, TacMedia.tracksKey, {
      configurable: true,
      value: Object.freeze({
        track: (name) => TacMedia.anyTrack(String(name)),
        changed: (hook) => TacMedia.changedHooks.push(hook),
        ended: (hook) => TacMedia.endedHooks.push(hook),
        speaker: () => TacMedia.speaker,
        attachFrames: (connection, transceiver) => TacMedia.attachIfEncrypted(String(connection), transceiver),
      }),
    })
    globalThis.navigator?.mediaDevices?.addEventListener?.('devicechange', () => TacMedia.devicesChanged())
    globalThis.addEventListener?.('pagehide', () => TacMedia.stopAll())
    globalThis.addEventListener?.('load', () => TacMedia.watchPeers())
    TacMedia.watchPeers()
    return true
  }

  static tracked(calls) {
    return Object.fromEntries(Object.entries(calls).map(([name, call]) => [name, (reply, ...values) => TacMedia.calling(reply, () => call(reply, ...values))]))
  }

  static calling(reply, work) {
    TacMedia.lastReply = reply
    TacMedia.watchPeers()
    return work()
  }

  static anyTrack(name) {
    return TacMedia.tracks.get(name)?.track ?? TacMedia.remote.get(name)?.track ?? null
  }

  static watchPeers() {
    const hooks = globalThis[TacMedia.peersHooksKey]
    if (TacMedia.peersHooked || !hooks) return
    TacMedia.peersHooked = true
    hooks.opened((connection, peer, reply) => TacMedia.opened(connection, peer, reply))
    hooks.closed((connection) => TacMedia.closed(connection))
  }

  static supported(feature) {
    if (feature === 'speakers' && TacMedia.bridged()) return true
    const [holder, member] = TacMedia.features[feature] ?? ['', '']
    if (feature === 'background') return globalThis.__tachyonMediaBackground === true
    return TacMedia.hasMember(holder === 'mediaDevices' ? globalThis.navigator?.mediaDevices : globalThis[holder], member)
  }

  static hasMember(holder, member) {
    if (member === '') return typeof holder === 'function'
    return typeof (holder?.prototype ?? holder)?.[member] === 'function'
  }

  static grant(call) {
    const granted = document.querySelector(TacMedia.grantTag)?.content.split(' ') ?? []
    return TacMedia.grants[call] ?? ['microphone', 'camera', 'screen'].find((name) => granted.includes(name)) ?? 'microphone'
  }

  static devices(reply, request) {
    TacMedia.listeners.add(reply)
    TacMedia.ask(() => Promise.all([TacMedia.mediaDevices().enumerateDevices(), TacMedia.hostSpeakers()]))
      .then(([devices, speakers]) => TacMedia.sendDevices(reply, String(request), TacMedia.withSpeakers(devices, speakers)))
      .catch((error) => reply(TacMedia.failure('', 'devices', error)))
    return null
  }

  static sendDevices(reply, request, devices) {
    const known = devices.filter((device) => device.kind in TacMedia.deviceKinds)
    const found = known.map((device, index) => TacMedia.found(request, device, index, known.length))
    for (const event of found) reply(event)
    if (known.length === 0) reply(['MediaDeviceFound', { request, id: '', kind: '', label: '', chosen: false, index: 0, count: 0 }])
  }

  static found(request, device, index, count) {
    const kind = TacMedia.deviceKinds[device.kind]
    const chosen = device.hostChosen ?? TacMedia.isChosen(kind, device.deviceId)
    return ['MediaDeviceFound', { request, id: device.deviceId, kind, label: device.label, chosen, index, count }]
  }

  static isChosen(kind, id) {
    if (kind === 'speaker') return id === (TacMedia.speaker || 'default')
    return [...TacMedia.tracks.values()].some((entry) => entry.track?.getSettings().deviceId === id)
  }

  static chooseSpeaker(reply, device) {
    const id = String(device ?? '')
    if (TacMedia.bridged()) return TacMedia.chooseHostSpeaker(reply, id)
    TacMedia.ensureSpeakers()
    TacMedia.speaker = id
    Promise.all(TacMedia.changedHooks.map((hook) => hook('speaker', null)).concat(TacMedia.players().map((audio) => TacMedia.toSpeaker(audio, id))))
      .then(() => reply(['MediaSpeakerChosen', { device: id }]))
      .catch((error) => reply(TacMedia.failure('', 'chooseSpeaker', error)))
    return null
  }

  static chooseHostSpeaker(reply, id) {
    TacMedia.host('chooseSpeaker', { device: id })
      .then(() => reply(['MediaSpeakerChosen', { device: id }]))
      .catch((error) => reply(TacMedia.failure('', 'chooseSpeaker', error)))
    return null
  }

  static hostSpeakers() {
    if (!TacMedia.bridged()) return null
    return TacMedia.host('speakers', {})
  }

  static withSpeakers(devices, speakers) {
    if (speakers === null) return devices
    const hosted = speakers.map((speaker) => ({ kind: 'audiooutput', deviceId: speaker.id, label: speaker.label, hostChosen: speaker.chosen === true }))
    return devices.filter((device) => device.kind !== 'audiooutput').concat(hosted)
  }

  static callActive(reply, on, title) {
    const values = { on: on === true, title: String(title ?? '') }
    TacMedia.ask(() => TacMedia.callState(values))
      .then((state) => reply(['MediaCallActive', { on: state.on === true, background: state.background === true }]))
      .catch((error) => reply(TacMedia.failure('', 'callActive', error)))
    return null
  }

  static callState(values) {
    if (!TacMedia.bridged()) return { on: values.on, background: false }
    return TacMedia.host('callActive', values)
  }

  static bridged() {
    return typeof globalThis.__tachyonNativeHostCall === 'function' && globalThis.__tachyonBrowserApiBridge?.includes(TacMedia.hostApi) === true
  }

  static async host(call, values) {
    const raw = await globalThis.__tachyonNativeHostCall(`browserApi.${TacMedia.hostApi}.${call}`, JSON.stringify(values))
    const answer = typeof raw === 'string' ? JSON.parse(raw) : raw
    if (!answer || 'error' in answer) throw new Error(answer?.error ?? 'The native app could not run that media call.')
    return answer.value
  }

  static ensureSpeakers() {
    if (typeof globalThis.HTMLMediaElement?.prototype.setSinkId !== 'function') throw new Error('Choosing a speaker is not available here (unsupported).')
  }

  static captureMicrophone(reply, track, device, processing) {
    const on = processing !== false
    const audio = { ...TacMedia.deviceConstraint(device), echoCancellation: on, noiseSuppression: on, autoGainControl: on }
    return TacMedia.capture(reply, 'captureMicrophone', TacMedia.newEntry(track, device, 'audio', { audio }))
  }

  static captureCamera(reply, track, device, quality) {
    const size = TacMedia.qualities[String(quality)] ?? TacMedia.qualities.medium
    const video = { ...TacMedia.deviceConstraint(device), ...size }
    return TacMedia.capture(reply, 'captureCamera', TacMedia.newEntry(track, device, 'video', { video }))
  }

  static captureScreen(reply, track, withAudio) {
    const entry = TacMedia.newEntry(track, '', 'screen', { video: true, audio: withAudio === true })
    TacMedia.claim(entry, reply)
    TacMedia.ask(() => TacMedia.mediaDevices().getDisplayMedia(entry.constraints))
      .then((stream) => TacMedia.screenStarted(entry, stream))
      .catch((error) => TacMedia.captureFailed(entry, 'captureScreen', error))
    return null
  }

  static screenStarted(entry, stream) {
    TacMedia.started(entry, stream.getVideoTracks()[0])
    const audio = stream.getAudioTracks()[0]
    if (audio) TacMedia.started(TacMedia.claim(TacMedia.newEntry(`${entry.name}.audio`, '', 'screen', {}), entry.reply), audio)
  }

  static newEntry(track, device, kind, constraints) {
    const name = TacMedia.checkedName(track)
    return { name, device: String(device ?? ''), kind, constraints, track: null, reply: null, muted: false }
  }

  static checkedName(track) {
    const name = String(track ?? '')
    if (!TacMedia.label.test(name)) throw new Error('A track name is 1 to 64 letters, digits, ".", "_" or "-" (invalid).')
    return name
  }

  static deviceConstraint(device) {
    const id = String(device ?? '')
    return id === '' ? {} : { deviceId: { exact: id } }
  }

  static capture(reply, call, entry) {
    TacMedia.claim(entry, reply)
    TacMedia.open(entry, call)
    return null
  }

  static claim(entry, reply) {
    TacMedia.ensureFree(entry.name)
    if (TacMedia.tracks.size >= TacMedia.mostTracks) throw new Error(`A page may hold at most ${TacMedia.mostTracks} local tracks.`)
    entry.reply = reply
    TacMedia.tracks.set(entry.name, entry)
    return entry
  }

  static ensureFree(name) {
    if (TacMedia.tracks.has(name)) throw new Error(`A track named ${name} already exists (exists).`)
  }

  static open(entry, call) {
    TacMedia.ask(() => TacMedia.mediaDevices().getUserMedia(entry.constraints))
      .then((stream) => TacMedia.started(entry, stream.getTracks()[0]))
      .catch((error) => TacMedia.captureFailed(entry, call, error))
  }

  static started(entry, track) {
    const previous = entry.track
    entry.track = track
    track.enabled = !entry.muted
    track.addEventListener('ended', () => TacMedia.trackStopped(entry, track))
    previous?.stop()
    TacMedia.announce(TacMedia.changedHooks, entry.name, track)
    entry.reply(TacMedia.startedEvent(entry, track))
  }

  static startedEvent(entry, track) {
    const settings = track.getSettings()
    const kind = track.kind
    return ['MediaTrackStarted', { track: entry.name, kind, device: settings.deviceId ?? '', label: track.label, width: settings.width ?? 0, height: settings.height ?? 0 }]
  }

  static captureFailed(entry, call, error) {
    TacMedia.tracks.delete(entry.name)
    entry.reply(TacMedia.failure(entry.name, call, error))
  }

  static trackStopped(entry, track) {
    if (entry.track !== track) return
    if (TacMedia.follows(entry)) return TacMedia.open(entry, 'follow')
    TacMedia.endedBy(entry, track.getSettings().deviceId ?? '')
  }

  static follows(entry) {
    return entry.device === '' && entry.kind !== 'screen' && entry.track !== null
  }

  static endedBy(entry, deviceId) {
    if (entry.kind === 'screen') return TacMedia.end(entry, 'ended')
    TacMedia.ask(() => TacMedia.mediaDevices().enumerateDevices())
      .then((devices) => TacMedia.end(entry, devices.some((device) => device.deviceId === deviceId) ? 'failed' : 'removed'))
      .catch(() => TacMedia.end(entry, 'failed'))
  }

  static end(entry, reason) {
    TacMedia.tracks.delete(entry.name)
    TacMedia.forgetLevel(entry.name)
    entry.track?.stop()
    TacMedia.announce(TacMedia.endedHooks, entry.name, null)
    entry.reply(['MediaTrackEnded', { track: entry.name, reason }])
  }

  static stopTrack(reply, track) {
    const entry = TacMedia.entry(track)
    entry.reply = reply
    TacMedia.end(entry, 'stopped')
    return null
  }

  static entry(track) {
    const entry = TacMedia.tracks.get(String(track))
    if (!entry) throw new Error(`There is no track named ${track} (unknown).`)
    return entry
  }

  static mute(track, muted) {
    const entry = TacMedia.entry(track)
    entry.muted = muted === true
    if (entry.kind === 'audio') entry.track.enabled = !entry.muted
    if (entry.kind === 'video') TacMedia.cameraOn(entry)
    return null
  }

  static cameraOn(entry) {
    if (!entry.muted) return TacMedia.open(entry, 'mute')
    const track = entry.track
    entry.track = null
    track?.stop()
    TacMedia.announce(TacMedia.changedHooks, entry.name, null)
  }

  static useFrameKey(reply, keyId, key) {
    TacMedia.ensurePageKeys()
    const id = TacMedia.checkedKeyId(keyId)
    TacMedia.frameReply = reply
    TacMedia.ask(() => TacMedia.handKey(id, key))
      .catch((error) => reply(TacMedia.failure('', 'useFrameKey', error)))
    return null
  }

  static handKey(id, name) {
    if (TacMedia.keySource() === 'native') return TacMedia.handNativeKey(id, name)
    return TacMedia.cryptoKey(name).then((cryptoKey) => TacMedia.worker().postMessage({ kind: 'use', keyId: id, key: cryptoKey }))
  }

  static async handNativeKey(id, name) {
    const publicKey = await TacMedia.bound(id)
    if (publicKey === null) return
    const wrapped = await globalThis[TacMedia.cryptoHooksKey].frameKey(String(name), id, publicKey).catch((error) => TacMedia.frameKeyRefused(error))
    TacMedia.worker().postMessage({ kind: 'wrapped', keyId: id, wrapped })
  }

  static bound(id) {
    return new Promise((resolve) => {
      TacMedia.binding.set(id, resolve)
      TacMedia.worker().postMessage({ kind: 'bind', keyId: id })
    })
  }

  static frameKeyRefused(error) {
    if (error?.reason === 'unknown') throw new Error('There is no Crypto key with that name (unknown).')
    throw new Error('The app could not hand the frame key to the media worker (unsupported).')
  }

  static postKey(reply, kind, keyId) {
    const id = TacMedia.checkedKeyId(keyId)
    TacMedia.frameReply = reply
    TacMedia.worker().postMessage({ kind, keyId: id })
    return null
  }

  static checkedKeyId(keyId) {
    const id = Number(keyId)
    if (!Number.isInteger(id) || id < 0 || id > TacMedia.largestKeyId) throw new Error('A frame key id is a whole number from 0 to 9007199254740991 (invalid).')
    return id
  }

  static async cryptoKey(name) {
    const hooks = globalThis[TacMedia.cryptoHooksKey]
    if (!hooks) throw new Error('Frame keys need the crypto browser API (unsupported).')
    return TacMedia.requireKey(await hooks.key(String(name)))
  }

  static requireKey(key) {
    if (!key) throw new Error('There is no Crypto key with that name (unknown).')
    return key
  }

  static worker() {
    if (TacMedia.frames) return TacMedia.frames
    TacMedia.ensureFrames()
    TacMedia.frames = new Worker(TacMedia.framesPath)
    TacMedia.frames.addEventListener('message', (event) => TacMedia.frameEvent(event.data))
    return TacMedia.frames
  }

  static ensurePageKeys() {
    if (TacMedia.keySource() === 'none') throw new Error('Frame encryption is not available in this app yet: its Crypto keys stay in native code (unsupported).')
  }

  static keySource() {
    if (globalThis.__tachyonBrowserApiBridge?.includes('crypto') !== true) return 'page'
    return globalThis.__tachyonNativeFrameKeys === true ? 'native' : 'none'
  }

  static ensureEncryption(on) {
    if (on === true) TacMedia.ensurePageKeys()
  }

  static ensureFrames() {
    if (typeof globalThis.RTCRtpScriptTransform !== 'function') throw new Error('Frame encryption is not available here (unsupported).')
  }

  static encryptFrames(reply, connection, on) {
    TacMedia.ensureEncryption(on)
    TacMedia.ensureFrames()
    const id = String(connection)
    const peer = TacMedia.peerFor(reply, id)
    TacMedia.frameReply = reply
    TacMedia.worker()
    TacMedia.markEncrypted(id, peer, on === true)
    for (const transceiver of peer.getTransceivers()) TacMedia.attachIfEncrypted(id, transceiver)
    return null
  }

  static peer(connection) {
    const peer = globalThis[TacMedia.peersHooksKey]?.connection(connection) ?? null
    if (!peer) throw new Error(`There is no peer connection named ${connection} (unknown).`)
    return peer
  }

  static peerFor(reply, connection) {
    const peer = TacMedia.peer(connection)
    TacMedia.replies.set(connection, reply)
    TacMedia.watch(connection, peer)
    return peer
  }

  static opened(connection, peer, reply) {
    TacMedia.replies.set(connection, reply)
    TacMedia.watch(connection, peer)
  }

  static watch(connection, peer) {
    if (TacMedia.watched.has(connection)) return
    TacMedia.watched.add(connection)
    peer.addEventListener('track', (event) => TacMedia.arrived(connection, event))
    peer.addEventListener('signalingstatechange', () => TacMedia.checkRemote(connection, peer))
  }

  static tell(connection, event) {
    const reply = TacMedia.replies.get(connection) ?? TacMedia.lastReply
    reply?.(event)
  }

  static send(reply, connection, track, stream, simulcast) {
    const id = String(connection)
    const peer = TacMedia.peerFor(reply, id)
    const entry = TacMedia.entry(track)
    TacMedia.ensureNotSent(id, entry.name)
    const transceiver = peer.addTransceiver(entry.track ?? TacMedia.mediaKind(entry), TacMedia.sendOptions(entry, stream, simulcast))
    TacMedia.sending.set(`${id} ${entry.name}`, { connection: id, name: entry.name, transceiver })
    TacMedia.attachIfEncrypted(id, transceiver)
    reply(['MediaSending', { connection: id, track: entry.name }])
    return null
  }

  static ensureNotSent(connection, name) {
    if (TacMedia.sending.has(`${connection} ${name}`)) throw new Error(`The track ${name} is already sent on ${connection} (exists).`)
  }

  static mediaKind(entry) {
    return entry.kind === 'audio' ? 'audio' : 'video'
  }

  static sendOptions(entry, stream, simulcast) {
    const options = { direction: 'sendrecv', streams: [TacMedia.namedStream(stream)] }
    if (entry.kind === 'screen' && entry.track?.kind === 'video') entry.track.contentHint = 'detail'
    if (simulcast === true && entry.kind === 'video') options.sendEncodings = TacMedia.layers.map((layer) => ({ ...layer }))
    return options
  }

  static namedStream(stream) {
    const name = String(stream ?? '')
    if (name !== '') TacMedia.checkedName(name)
    if (!TacMedia.streams.has(name)) TacMedia.streams.set(name, new MediaStream())
    return TacMedia.streams.get(name)
  }

  static replace(reply, connection, track, withTrack) {
    const id = String(connection)
    TacMedia.peerFor(reply, id)
    const sent = TacMedia.sent(id, track)
    const next = TacMedia.entry(withTrack)
    TacMedia.ensureSameKind(sent, next)
    TacMedia.ensureNotSent(id, next.name)
    TacMedia.sending.delete(`${id} ${sent.name}`)
    TacMedia.sending.set(`${id} ${next.name}`, { ...sent, name: next.name })
    TacMedia.ask(() => sent.transceiver.sender.replaceTrack(next.track)).catch((error) => reply(TacMedia.failure(next.name, 'replace', error)))
    return null
  }

  static ensureSameKind(sent, next) {
    if (TacMedia.mediaKind(next) !== sent.transceiver.receiver.track.kind) throw new Error('A track can only replace a track of the same kind (invalid).')
  }

  static unsend(reply, connection, track) {
    const id = String(connection)
    const peer = TacMedia.peerFor(reply, id)
    const sent = TacMedia.sent(id, track)
    TacMedia.sending.delete(`${id} ${sent.name}`)
    peer.removeTrack(sent.transceiver.sender)
    return null
  }

  static sent(connection, track) {
    const sent = TacMedia.sending.get(`${connection} ${track}`)
    if (!sent) throw new Error(`The track ${track} is not sent on ${connection} (unknown).`)
    return sent
  }

  static arrived(connection, event) {
    const transceiver = event.transceiver
    TacMedia.attachIfEncrypted(connection, transceiver)
    const name = `${connection}.${transceiver.mid}`
    if (TacMedia.remote.has(name)) return
    if (TacMedia.remote.size >= TacMedia.mostRemote) return TacMedia.tell(connection, TacMedia.failure(name, 'receive', new Error('Too many remote tracks.')))
    const track = transceiver.receiver.track
    TacMedia.remote.set(name, { name, connection, track, transceiver, audio: TacMedia.player(track) })
    track.addEventListener('mute', () => TacMedia.tell(connection, ['MediaTrackMuted', { track: name, muted: true }]))
    track.addEventListener('unmute', () => TacMedia.tell(connection, ['MediaTrackMuted', { track: name, muted: false }]))
    TacMedia.announce(TacMedia.changedHooks, name, track)
    TacMedia.tell(connection, ['MediaTrackArrived', { connection, track: name, kind: track.kind, stream: event.streams?.[0]?.id ?? '' }])
  }

  static checkRemote(connection, peer) {
    if (peer.signalingState !== 'stable') return
    const gone = [...TacMedia.remote.values()].filter((entry) => entry.connection === connection && TacMedia.notReceiving.includes(entry.transceiver.currentDirection))
    for (const entry of gone) TacMedia.endRemote(entry, 'remote')
  }

  static endRemote(entry, reason) {
    TacMedia.remote.delete(entry.name)
    TacMedia.stopPlayer(entry.audio)
    TacMedia.forgetLevel(entry.name)
    TacMedia.announce(TacMedia.endedHooks, entry.name, null)
    TacMedia.tell(entry.connection, ['MediaTrackEnded', { track: entry.name, reason }])
  }

  static closed(connection) {
    const remote = [...TacMedia.remote.values()].filter((entry) => entry.connection === connection)
    for (const entry of remote) TacMedia.endRemote(entry, 'closed')
    for (const [key, sent] of TacMedia.sending) {
      if (sent.connection === connection) TacMedia.sending.delete(key)
    }
    TacMedia.watched.delete(connection)
    TacMedia.encrypted.delete(connection)
    TacMedia.replies.delete(connection)
  }

  static player(track) {
    if (track.kind !== 'audio' || typeof globalThis.document?.createElement !== 'function') return null
    const audio = document.createElement('audio')
    audio.autoplay = true
    audio.srcObject = new MediaStream([track])
    if (TacMedia.speaker !== '') TacMedia.toSpeaker(audio, TacMedia.speaker)?.catch?.(() => null)
    TacMedia.play(audio)
    return audio
  }

  static players() {
    return [...TacMedia.remote.values()].map((entry) => entry.audio).filter((audio) => audio !== null)
  }

  static toSpeaker(audio, id) {
    if (typeof audio.setSinkId === 'function') return audio.setSinkId(id)
    return null
  }

  static play(audio) {
    TacMedia.ask(() => audio.play())
      .then(() => TacMedia.blocked.delete(audio))
      .catch((error) => TacMedia.audioBlocked(audio, error))
  }

  static audioBlocked(audio, error) {
    if (error?.name !== 'NotAllowedError') return
    const first = TacMedia.blocked.size === 0
    TacMedia.blocked.add(audio)
    if (first) TacMedia.lastReply?.(['MediaAudioBlocked', {}])
  }

  static resumeAudio() {
    for (const audio of [...TacMedia.blocked]) TacMedia.play(audio)
    return null
  }

  static stopPlayer(audio) {
    if (!audio) return
    audio.pause?.()
    audio.srcObject = null
    TacMedia.blocked.delete(audio)
  }

  static stats(reply, request, connection) {
    const id = String(connection)
    const peer = TacMedia.peerFor(reply, id)
    TacMedia.ask(() => peer.getStats())
      .then((report) => TacMedia.sendStats(reply, String(request), id, report))
      .catch((error) => reply(TacMedia.failure('', 'stats', error)))
    return null
  }

  static sendStats(reply, request, connection, report) {
    const streams = [...report.values()].filter((item) => item.type === 'inbound-rtp' || item.type === 'outbound-rtp')
    const rows = streams.map((item) => TacMedia.statRow(connection, item, report)).filter((row) => row.track !== '')
    rows.forEach((row, index) => reply(['MediaStats', { request, connection, ...row, index, count: rows.length }]))
    if (rows.length === 0) reply(['MediaStats', { request, connection, ...TacMedia.emptyStat(), index: 0, count: 0 }])
  }

  static emptyStat() {
    return { track: '', layer: '', direction: '', kind: '', bitrate: 0, packetsLost: 0, lossPercent: 0, jitterMs: 0, roundTripMs: 0, framesPerSecond: 0, width: 0, height: 0 }
  }

  static statRow(connection, item, report) {
    const inbound = item.type === 'inbound-rtp'
    const far = inbound ? item : report.get(item.remoteId)
    return {
      track: inbound ? TacMedia.receivedName(connection, item) : TacMedia.sentName(connection, item, report),
      layer: item.rid ?? '',
      direction: inbound ? 'receive' : 'send',
      kind: item.kind ?? item.mediaType ?? '',
      bitrate: TacMedia.bitrate(item, inbound ? item.bytesReceived : item.bytesSent),
      packetsLost: far?.packetsLost ?? 0,
      lossPercent: inbound ? TacMedia.receivedLoss(item) : Math.round((far?.fractionLost ?? 0) * 1000) / 10,
      jitterMs: TacMedia.milliseconds(far?.jitter),
      roundTripMs: TacMedia.milliseconds(inbound ? TacMedia.pairRoundTrip(report) : far?.roundTripTime),
      framesPerSecond: Math.round(item.framesPerSecond ?? 0),
      width: item.frameWidth ?? 0,
      height: item.frameHeight ?? 0,
    }
  }

  static receivedName(connection, item) {
    const entry = [...TacMedia.remote.values()].find((remote) => remote.connection === connection && (remote.transceiver.mid === item.mid || remote.track.id === item.trackIdentifier))
    return entry?.name ?? ''
  }

  static sentName(connection, item, report) {
    const source = report.get(item.mediaSourceId)?.trackIdentifier
    const sent = [...TacMedia.sending.values()].find((entry) => entry.connection === connection && (entry.transceiver.mid === item.mid || entry.transceiver.sender.track?.id === source))
    return sent?.name ?? ''
  }

  static bitrate(item, bytes) {
    const before = TacMedia.previousStats.get(item.id)
    TacMedia.previousStats.set(item.id, { bytes: bytes ?? 0, time: item.timestamp })
    if (!before || item.timestamp <= before.time) return 0
    return Math.round(((bytes ?? 0) - before.bytes) * 8000 / (item.timestamp - before.time))
  }

  static receivedLoss(item) {
    const lost = item.packetsLost ?? 0
    const total = lost + (item.packetsReceived ?? 0)
    return total > 0 ? Math.round(lost * 1000 / total) / 10 : 0
  }

  static milliseconds(seconds) {
    return Math.round((seconds ?? 0) * 1000)
  }

  static pairRoundTrip(report) {
    return [...report.values()].find((item) => item.type === 'candidate-pair' && item.nominated && item.state === 'succeeded')?.currentRoundTripTime
  }

  static speaking(reply, on) {
    TacMedia.speakReply = reply
    clearInterval(TacMedia.levelTimer)
    TacMedia.levelTimer = on === true ? setInterval(() => TacMedia.measure(), TacMedia.levelMs) : 0
    if (on !== true) TacMedia.levels.clear()
    return null
  }

  static measure() {
    for (const [name, level] of TacMedia.currentLevels()) TacMedia.judge(name, level)
    TacMedia.pickLoudest()
  }

  static currentLevels() {
    const remote = [...TacMedia.remote.values()].filter((entry) => entry.track.kind === 'audio').map((entry) => [entry.name, TacMedia.remoteLevel(entry)])
    const local = [...TacMedia.tracks.values()].filter((entry) => entry.kind === 'audio' && entry.track).map((entry) => [entry.name, TacMedia.localLevel(entry)])
    return remote.concat(local)
  }

  static remoteLevel(entry) {
    const sources = entry.transceiver.receiver.getSynchronizationSources?.() ?? []
    return Math.max(0, ...sources.map((source) => source.audioLevel ?? 0))
  }

  static localLevel(entry) {
    const analyser = TacMedia.analyser(entry)
    if (!analyser) return 0
    const samples = new Float32Array(analyser.fftSize)
    analyser.getFloatTimeDomainData(samples)
    return Math.sqrt(samples.reduce((sum, value) => sum + value * value, 0) / samples.length)
  }

  static analyser(entry) {
    const known = TacMedia.analysers.get(entry.name)
    if (known?.track === entry.track) return known.node
    if (typeof globalThis.AudioContext !== 'function') return null
    TacMedia.audioContext ??= new AudioContext()
    const node = TacMedia.audioContext.createAnalyser()
    TacMedia.audioContext.createMediaStreamSource(new MediaStream([entry.track])).connect(node)
    TacMedia.analysers.set(entry.name, { track: entry.track, node })
    return node
  }

  static judge(name, level) {
    const state = TacMedia.levels.get(name) ?? { speaking: false, count: 0, level: 0 }
    const crossing = state.speaking ? level < TacMedia.speakingOff : level >= TacMedia.speakingOn
    state.level = level
    state.count = crossing ? state.count + 1 : 0
    TacMedia.levels.set(name, state)
    if (state.count < (state.speaking ? TacMedia.quietTicks : TacMedia.speakingTicks)) return
    state.speaking = !state.speaking
    state.count = 0
    TacMedia.speakReply?.(['MediaSpeaking', { track: name, speaking: state.speaking }])
  }

  static pickLoudest() {
    const top = TacMedia.topSpeaker()
    TacMedia.loudestTicks = top === TacMedia.loudestCandidate ? TacMedia.loudestTicks + 1 : 1
    TacMedia.loudestCandidate = top
    if (top === '' || top === TacMedia.loudest || TacMedia.loudestTicks < TacMedia.speakingTicks) return
    TacMedia.loudest = top
    TacMedia.speakReply?.(['MediaLoudest', { track: top }])
  }

  static topSpeaker() {
    const speaking = [...TacMedia.levels].filter(([, state]) => state.speaking)
    return speaking.sort((first, second) => second[1].level - first[1].level)[0]?.[0] ?? ''
  }

  static forgetLevel(name) {
    TacMedia.levels.delete(name)
    TacMedia.analysers.delete(name)
  }

  static markEncrypted(connection, peer, on) {
    if (on) return TacMedia.encrypted.set(connection, peer)
    TacMedia.encrypted.delete(connection)
  }

  static attachIfEncrypted(connection, transceiver) {
    const on = TacMedia.encrypted.has(connection)
    const kind = transceiver.receiver.track.kind
    if (on) TacMedia.preferVp8(transceiver, kind)
    transceiver.sender.transform = on ? new RTCRtpScriptTransform(TacMedia.frames, { side: 'send', kind }) : null
    transceiver.receiver.transform = on ? new RTCRtpScriptTransform(TacMedia.frames, { side: 'receive', kind, track: `${connection}.${transceiver.mid}` }) : null
  }

  static preferVp8(transceiver, kind) {
    if (kind !== 'video' || typeof transceiver.setCodecPreferences !== 'function') return
    const codecs = RTCRtpReceiver.getCapabilities('video')?.codecs ?? []
    const vp8 = codecs.filter((codec) => codec.mimeType.toLowerCase() === TacMedia.vp8)
    if (vp8.length > 0) transceiver.setCodecPreferences(vp8)
  }

  static frameEvent(message) {
    if (message.kind === 'ready') return TacMedia.frameReply?.(['MediaFrameKeyReady', { keyId: message.keyId }])
    if (message.kind === 'rejected') return TacMedia.frameRejected(message.track)
    if (message.kind === 'missing') return TacMedia.frameMissing(message.track, message.keyId)
    if (message.kind === 'bound') return TacMedia.frameBound(message.keyId, message.publicKey)
    if (message.kind === 'refused') return TacMedia.frameRefused(message.keyId)
    TacMedia.frameReply?.(['MediaFailed', { track: '', call: 'sendFrameKey', reason: 'failed', message: TacMedia.messages.exhausted }])
  }

  static frameBound(keyId, publicKey) {
    const resolve = TacMedia.binding.get(keyId) ?? (() => null)
    TacMedia.binding.delete(keyId)
    resolve(publicKey)
  }

  static frameRefused(keyId) {
    TacMedia.frameBound(keyId, null)
    TacMedia.frameReply?.(TacMedia.failure('', 'useFrameKey', new Error('The media worker refused the frame key (unsupported).')))
  }

  static frameMissing(track, keyId) {
    if (!TacMedia.dueReport(`missing ${track} ${keyId}`)) return
    TacMedia.frameReply?.(['MediaFrameKeyMissing', { track, keyId }])
  }

  static frameRejected(track) {
    const count = (TacMedia.rejected.get(track) ?? 0) + 1
    TacMedia.rejected.set(track, count)
    if (!TacMedia.dueReport(`rejected ${track}`)) return
    TacMedia.rejected.delete(track)
    TacMedia.frameReply?.(['MediaFrameRejected', { track, count }])
  }

  static dueReport(name) {
    const now = Date.now()
    if (now - (TacMedia.reported.get(name) ?? -Infinity) < TacMedia.reportGapMs) return false
    TacMedia.reported.set(name, now)
    return true
  }

  static devicesChanged() {
    for (const reply of TacMedia.listeners) reply(['MediaDevicesChanged', {}])
    TacMedia.ask(() => TacMedia.mediaDevices().enumerateDevices())
      .then((devices) => TacMedia.followDefaults(devices))
      .catch(() => null)
  }

  static followDefaults(devices) {
    const moved = [...TacMedia.tracks.values()].filter((entry) => TacMedia.defaultMoved(entry, devices))
    for (const entry of moved) TacMedia.open(entry, 'follow')
  }

  static defaultMoved(entry, devices) {
    if (!TacMedia.follows(entry)) return false
    const kind = entry.kind === 'audio' ? 'audioinput' : 'videoinput'
    const preferred = devices.find((device) => device.kind === kind && device.deviceId === 'default')
    return preferred !== undefined && preferred.groupId !== entry.track.getSettings().groupId
  }

  static announce(hooks, name, track) {
    for (const hook of hooks) hook(name, track)
    TacMedia.resend(name, track)
  }

  static resend(name, track) {
    for (const sent of TacMedia.sending.values()) {
      if (sent.name === name) TacMedia.ask(() => sent.transceiver.sender.replaceTrack(track)).catch(() => null)
    }
  }

  static failure(track, call, error) {
    const reason = TacMedia.reasons[error?.name] ?? TacMedia.reasonIn(error)
    return ['MediaFailed', { track, call, reason, message: TacMedia.messageFor(call, reason) }]
  }

  static messageFor(call, reason) {
    return TacMedia.callMessages[`${call} ${reason}`] ?? TacMedia.messages[reason] ?? TacMedia.messages.failed
  }

  static reasonIn(error) {
    const word = /\((unsupported|unknown|invalid|exists)\)/.exec(String(error?.message ?? ''))
    return word ? word[1] : 'failed'
  }

  static ask(work) {
    return new Promise((resolve) => resolve(work()))
  }

  static mediaDevices() {
    const devices = globalThis.navigator?.mediaDevices
    if (!devices) throw new TypeError('Capturing audio or video is not available here (unsupported).')
    return devices
  }

  static stopAll() {
    for (const entry of TacMedia.tracks.values()) entry.track?.stop()
    for (const audio of TacMedia.players()) TacMedia.stopPlayer(audio)
    clearInterval(TacMedia.levelTimer)
    TacMedia.tracks.clear()
    TacMedia.remote.clear()
  }
}
