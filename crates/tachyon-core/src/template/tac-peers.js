class TacPeers {
  static key = Symbol.for('tachyon.tac.peers')
  static hooksKey = Symbol.for('tachyon.tac.peers.hooks')
  static settingsTag = 'meta[name="tachyon-peers"]'
  static mostConnections = 32
  static mostChannels = 16
  static mostMessageBytes = 32768
  static mostDescriptionBytes = 262144
  static mostCandidateBytes = 65536
  static mostLabelLength = 64
  static mostWaitingBytes = 1048576
  static readyBytes = 262144
  static base64url = /^[A-Za-z0-9_-]*$/
  static label = /^[A-Za-z0-9_.-]{1,64}$/
  static turnSchemes = ['turn:', 'turns:']
  static states = ['new', 'connecting', 'connected', 'disconnected', 'failed', 'closed']
  static reasons = Object.freeze({
    InvalidAccessError: 'invalid',
    InvalidStateError: 'invalid',
    InvalidModificationError: 'invalid',
    OperationError: 'invalid',
    SyntaxError: 'invalid',
    TypeError: 'invalid',
    TacDescriptionTooLarge: 'tooLarge',
  })
  static messages = Object.freeze({
    invalid: 'The description, candidate or message is not valid for this connection.',
    tooLarge: 'A message from the other side is larger than 32768 bytes; the channel was closed.',
    failed: 'The peer connection call failed.',
    TacDescriptionTooLarge: 'This connection made a description larger than 262144 bytes; offer fewer tracks on one connection.',
  })
  static connections = new Map()
  static openedHooks = []
  static closedHooks = []
  static counter = 0
  static installed = TacPeers.install()

  static install() {
    Object.defineProperty(globalThis, TacPeers.key, {
      configurable: true,
      value: Object.freeze({
        supported: () => TacPeers.supported(),
        calls: Object.freeze(['open', 'offer', 'answer', 'accept', 'addCandidate', 'restart', 'openChannel', 'send', 'closeChannel', 'close']),
        open: (reply, username, credential) => TacPeers.open(reply, username, credential),
        offer: (reply, connection) => TacPeers.offer(connection),
        answer: (reply, connection, sdp) => TacPeers.answer(connection, sdp),
        accept: (reply, connection, sdp) => TacPeers.accept(connection, sdp),
        addCandidate: (reply, connection, candidate) => TacPeers.addCandidate(connection, candidate),
        restart: (reply, connection) => TacPeers.restart(connection),
        openChannel: (reply, connection, label, ordered) => TacPeers.openChannel(connection, label, ordered),
        send: (reply, connection, channel, data) => TacPeers.send(connection, channel, data),
        closeChannel: (reply, connection, channel) => TacPeers.closeChannel(connection, channel),
        close: (reply, connection) => TacPeers.close(connection),
      }),
    })
    Object.defineProperty(globalThis, TacPeers.hooksKey, {
      configurable: true,
      value: Object.freeze({
        connection: (connection) => TacPeers.connections.get(String(connection))?.peer ?? null,
        opened: (hook) => TacPeers.openedHooks.push(hook),
        closed: (hook) => TacPeers.closedHooks.push(hook),
      }),
    })
    globalThis.addEventListener?.('pagehide', () => TacPeers.closeAll())
    return true
  }

  static supported() {
    return typeof globalThis.RTCPeerConnection === 'function'
  }

  static settings() {
    const content = document.querySelector(TacPeers.settingsTag)?.content ?? '{}'
    const settings = JSON.parse(content)
    return { iceServers: settings.iceServers ?? [], relayOnly: settings.relayOnly === true }
  }

  static configuration(username, credential) {
    const settings = TacPeers.settings()
    const iceServers = settings.iceServers.map((url) => TacPeers.server(url, String(username ?? ''), String(credential ?? '')))
    return { iceServers, iceTransportPolicy: settings.relayOnly ? 'relay' : 'all' }
  }

  static server(url, username, credential) {
    const relay = TacPeers.turnSchemes.some((scheme) => url.startsWith(scheme))
    return relay && username !== '' ? { urls: url, username, credential } : { urls: url }
  }

  static open(reply, username, credential) {
    TacPeers.ensureSupported()
    TacPeers.ensureRoom(TacPeers.connections.size, TacPeers.mostConnections, 'connections')
    TacPeers.counter += 1
    const id = `peer-${TacPeers.counter}`
    const peer = TacPeers.connection(TacPeers.configuration(username, credential))
    const entry = { id, peer, reply, channels: new Map(), counter: 0 }
    TacPeers.connections.set(id, entry)
    TacPeers.announce(TacPeers.openedHooks, entry)
    TacPeers.listen(entry)
    return id
  }

  static connection(configuration) {
    try {
      return new RTCPeerConnection(configuration)
    } catch (error) {
      return TacPeers.withoutTransport(configuration, error)
    }
  }

  static withoutTransport(configuration, error) {
    if (error?.name !== 'SyntaxError') throw error
    const iceServers = configuration.iceServers.map((server) => ({ ...server, urls: server.urls.split('?')[0] }))
    return new RTCPeerConnection({ ...configuration, iceServers })
  }

  static announce(hooks, entry) {
    for (const hook of hooks) hook(entry.id, entry.peer, entry.reply)
  }

  static ensureSupported() {
    if (!TacPeers.supported()) throw new Error('Peer connections are not available here (unsupported).')
  }

  static ensureRoom(count, most, what) {
    if (count >= most) throw new Error(`A page may hold at most ${most} peer ${what}.`)
  }

  static listen(entry) {
    entry.peer.addEventListener('icecandidate', (event) => TacPeers.emit(entry, ['PeerCandidate', { connection: entry.id, candidateJson: TacPeers.candidateText(event.candidate) }]))
    entry.peer.addEventListener('connectionstatechange', () => TacPeers.stateChanged(entry))
    entry.peer.addEventListener('negotiationneeded', () => TacPeers.emit(entry, ['PeerNegotiationNeeded', { connection: entry.id }]))
    entry.peer.addEventListener('datachannel', (event) => TacPeers.adopt(entry, event.channel))
  }

  static candidateText(candidate) {
    return candidate ? JSON.stringify(candidate.toJSON()) : ''
  }

  static stateChanged(entry) {
    const state = entry.peer.connectionState
    if (TacPeers.states.includes(state)) TacPeers.emit(entry, ['PeerState', { connection: entry.id, state }])
  }

  static entry(connection) {
    const entry = TacPeers.connections.get(String(connection))
    if (!entry) throw new Error(`No open peer connection is named ${TacPeers.shown(connection)}.`)
    return entry
  }

  static shown(value) {
    return JSON.stringify(String(value).slice(0, TacPeers.mostLabelLength))
  }

  static offer(connection) {
    const entry = TacPeers.entry(connection)
    TacPeers.describe(entry, 'offer', () => entry.peer.createOffer())
    return null
  }

  static restart(connection) {
    const entry = TacPeers.entry(connection)
    TacPeers.describe(entry, 'restart', () => entry.peer.createOffer({ iceRestart: true }))
    return null
  }

  static answer(connection, sdp) {
    const entry = TacPeers.entry(connection)
    const offer = { type: 'offer', sdp: TacPeers.incomingDescription(sdp) }
    TacPeers.describe(entry, 'answer', () => entry.peer.setRemoteDescription(offer).then(() => entry.peer.createAnswer()))
    return null
  }

  static describe(entry, call, made) {
    TacPeers.later(entry, call, made().then((description) => entry.peer.setLocalDescription(description)).then(() => TacPeers.description(entry)))
  }

  static description(entry) {
    const local = entry.peer.localDescription
    return ['PeerDescription', { connection: entry.id, kind: local.type, sdp: TacPeers.localText(local.sdp) }]
  }

  static localText(sdp) {
    if (new TextEncoder().encode(sdp).length <= TacPeers.mostDescriptionBytes) return sdp
    throw Object.assign(new Error('description too large'), { name: 'TacDescriptionTooLarge' })
  }

  static accept(connection, sdp) {
    const entry = TacPeers.entry(connection)
    const answer = { type: 'answer', sdp: TacPeers.incomingDescription(sdp) }
    TacPeers.later(entry, 'accept', entry.peer.setRemoteDescription(answer).then(() => null))
    return null
  }

  static addCandidate(connection, candidate) {
    const entry = TacPeers.entry(connection)
    const text = TacPeers.incomingCandidate(candidate)
    TacPeers.later(entry, 'addCandidate', Promise.resolve().then(() => entry.peer.addIceCandidate(TacPeers.candidate(text))).then(() => null))
    return null
  }

  static candidate(text) {
    if (text === '') return { candidate: '' }
    const parsed = JSON.parse(text)
    if (parsed === null || typeof parsed !== 'object' || typeof parsed.candidate !== 'string') throw new TypeError('not a candidate')
    return parsed
  }

  static incomingDescription(value) {
    return TacPeers.limited(value, TacPeers.mostDescriptionBytes, `A description is at most ${TacPeers.mostDescriptionBytes} bytes; offer fewer tracks on one connection.`)
  }

  static incomingCandidate(value) {
    return TacPeers.limited(value, TacPeers.mostCandidateBytes, `A candidate is at most ${TacPeers.mostCandidateBytes} bytes.`)
  }

  static limited(value, most, message) {
    const text = String(value ?? '')
    if (new TextEncoder().encode(text).length > most) throw new Error(message)
    return text
  }

  static later(entry, call, work) {
    work.then((event) => TacPeers.answered(entry, event), (error) => TacPeers.failed(entry, call, error))
  }

  static answered(entry, event) {
    if (event) TacPeers.emit(entry, event)
  }

  static emit(entry, event) {
    if (TacPeers.connections.has(entry.id)) entry.reply(event)
  }

  static failed(entry, call, error) {
    const name = String(error?.name ?? '')
    TacPeers.failure(entry, call, TacPeers.reasons[name] ?? 'failed', TacPeers.messages[name])
  }

  static failure(entry, call, reason, message = TacPeers.messages[reason]) {
    TacPeers.emit(entry, ['PeerFailed', { connection: entry.id, call, reason, message }])
  }

  static openChannel(connection, label, ordered) {
    const entry = TacPeers.entry(connection)
    const name = TacPeers.channelLabel(label)
    TacPeers.ensureRoom(entry.channels.size, TacPeers.mostChannels, 'channels on one connection')
    const options = ordered === true ? { ordered: true } : { ordered: false, maxRetransmits: 0 }
    return TacPeers.adopt(entry, entry.peer.createDataChannel(name, options), false)
  }

  static channelLabel(label) {
    const name = String(label ?? '')
    if (!TacPeers.label.test(name)) throw new Error('A channel label is 1 to 64 letters, digits, dots, dashes or underscores.')
    return name
  }

  static adopt(entry, channel, remote = true) {
    entry.counter += 1
    const id = `channel-${entry.counter}`
    const held = { id, channel, waiting: false }
    channel.binaryType = 'arraybuffer'
    channel.bufferedAmountLowThreshold = TacPeers.readyBytes
    entry.channels.set(id, held)
    channel.addEventListener('open', () => TacPeers.emit(entry, ['PeerChannelOpened', { connection: entry.id, channel: id, label: channel.label, ordered: channel.ordered, remote }]))
    channel.addEventListener('message', (event) => TacPeers.received(entry, held, event.data))
    channel.addEventListener('bufferedamountlow', () => TacPeers.drained(entry, held))
    channel.addEventListener('close', () => TacPeers.channelClosed(entry, held))
    return id
  }

  static received(entry, held, data) {
    const bytes = typeof data === 'string' ? new TextEncoder().encode(data) : new Uint8Array(data)
    if (bytes.byteLength > TacPeers.mostMessageBytes) return TacPeers.refuseLarge(entry, held)
    TacPeers.emit(entry, ['PeerMessage', { connection: entry.id, channel: held.id, data: TacPeers.encoded(bytes) }])
    return null
  }

  static refuseLarge(entry, held) {
    TacPeers.failure(entry, 'receive', 'tooLarge')
    held.channel.close()
    return null
  }

  static drained(entry, held) {
    if (!held.waiting) return
    held.waiting = false
    TacPeers.emit(entry, ['PeerChannelReady', { connection: entry.id, channel: held.id }])
  }

  static channelClosed(entry, held) {
    if (!entry.channels.delete(held.id)) return
    TacPeers.emit(entry, ['PeerChannelClosed', { connection: entry.id, channel: held.id }])
  }

  static held(connection, channel) {
    const held = TacPeers.entry(connection).channels.get(String(channel))
    if (!held) throw new Error(`No open channel is named ${TacPeers.shown(channel)} on this connection.`)
    return held
  }

  static send(connection, channel, data) {
    const held = TacPeers.held(connection, channel)
    const bytes = TacPeers.decoded(data)
    TacPeers.ensureOpen(held.channel)
    return TacPeers.queued(held, bytes)
  }

  static ensureOpen(channel) {
    if (channel.readyState !== 'open') throw new Error('The channel is not open yet. Wait for PeerChannelOpened.')
  }

  static queued(held, bytes) {
    held.waiting = held.channel.bufferedAmount + bytes.byteLength > TacPeers.mostWaitingBytes
    if (held.waiting) return false
    held.channel.send(bytes)
    return true
  }

  static decoded(text) {
    const value = String(text ?? '')
    if (!TacPeers.base64url.test(value) || value.length % 4 === 1) throw new Error('Peer data is base64url text without padding.')
    const binary = atob(value.replace(/-/g, '+').replace(/_/g, '/'))
    if (binary.length > TacPeers.mostMessageBytes) throw new Error(`A peer message is at most ${TacPeers.mostMessageBytes} bytes.`)
    return Uint8Array.from(binary, (letter) => letter.charCodeAt(0))
  }

  static encoded(bytes) {
    const binary = Array.from(bytes, (byte) => String.fromCharCode(byte)).join('')
    return btoa(binary).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '')
  }

  static closeChannel(connection, channel) {
    TacPeers.held(connection, channel).channel.close()
    return null
  }

  static close(connection) {
    const entry = TacPeers.entry(connection)
    TacPeers.connections.delete(entry.id)
    entry.channels.clear()
    TacPeers.announce(TacPeers.closedHooks, entry)
    entry.peer.close()
    entry.reply(['PeerState', { connection: entry.id, state: 'closed' }])
    return null
  }

  static closeAll() {
    for (const entry of TacPeers.connections.values()) TacPeers.announce(TacPeers.closedHooks, entry)
    for (const entry of TacPeers.connections.values()) entry.peer.close()
    TacPeers.connections.clear()
  }
}
