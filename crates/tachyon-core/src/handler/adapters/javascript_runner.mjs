import { spawn } from 'node:child_process'
import net from 'node:net'
import { pathToFileURL } from 'node:url'

class ProtocolLimits {
  static maxFrameBytes = 16 * 1024 * 1024
  static maxRelayStdoutBytes = 16 * 1024 * 1024
  static maxRelayStderrBytes = 64 * 1024
  static maxResourceBytes = 1024 * 1024
  static maxResourceHeadBytes = 16 * 1024
  static resourceTimeoutMs = 5000
  static scrollTimeoutMs = 30_000
  static gatewayAnswerMarginMs = 1000
}

class RelaySettings {
  static defaultDeadlineMs = 30_000
  static failureMessage = 'Component invocation failed.'
}

function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
}

function resourcePort() {
  const endpoint = /^http:\/\/127\.0\.0\.1:([0-9]{1,5})\/$/.exec(process.env.YON_RESOURCE_ENDPOINT ?? '')
  if (!endpoint) throw new Error()
  const port = Number(endpoint[1])
  if (port < 1 || port > 65535) throw new Error()
  return port
}

function resourceToken() {
  const token = process.env.YON_RESOURCE_TOKEN ?? ''
  if (!/^[a-f0-9]{64}$/.test(token)) throw new Error()
  return token
}

function resourceRequest(port, token, body) {
  const head = [
    'POST / HTTP/1.1',
    `Host: 127.0.0.1:${port}`,
    `Authorization: Bearer ${token}`,
    'Content-Type: application/json',
    `Content-Length: ${body.length}`,
    'Connection: close',
    '',
    '',
  ].join('\r\n')
  return Buffer.concat([Buffer.from(head, 'latin1'), body])
}

function resourceContentLength(lines) {
  let length = -1
  for (const line of lines) {
    const lower = line.toLowerCase()
    if (lower.startsWith('transfer-encoding:')) throw new Error()
    if (!lower.startsWith('content-length:')) continue
    const value = lower.slice('content-length:'.length).trim()
    if (length >= 0 || !/^[0-9]{1,7}$/.test(value)) throw new Error()
    length = Number(value)
  }
  if (length < 0 || length > ProtocolLimits.maxResourceBytes) throw new Error()
  return length
}

function completeResourceBody(received) {
  const headEnd = received.indexOf('\r\n\r\n')
  if (headEnd < 0) {
    if (received.length > ProtocolLimits.maxResourceHeadBytes) throw new Error()
    return null
  }
  const lines = received.subarray(0, headEnd).toString('latin1').split('\r\n')
  if (!/^HTTP\/1\.[01] 200( |$)/.test(lines[0])) throw new Error()
  const length = resourceContentLength(lines.slice(1))
  const body = received.subarray(headEnd + 4)
  if (body.length < length) return null
  return body.subarray(0, length)
}

function exchangeWithEndpoint(port, request, timeoutMs) {
  return new Promise((resolve, reject) => {
    const socket = net.connect({ host: '127.0.0.1', port })
    let received = Buffer.alloc(0)
    let settled = false
    const settle = (error, body) => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      socket.destroy()
      if (error) reject(error)
      else resolve(body)
    }
    const timer = setTimeout(() => settle(new Error()), timeoutMs)
    socket.on('connect', () => socket.write(request))
    socket.on('data', (chunk) => {
      received = Buffer.concat([received, chunk])
      try {
        const body = completeResourceBody(received)
        if (body !== null) settle(null, body)
      } catch (error) {
        settle(error)
      }
    })
    socket.on('end', () => settle(new Error()))
    socket.on('error', (error) => settle(error))
  })
}

async function postResource(body, timeoutMs) {
  const port = resourcePort()
  const token = resourceToken()
  const bytes = Buffer.from(body, 'utf8')
  if (bytes.length > ProtocolLimits.maxResourceBytes) throw new Error()
  const answer = await exchangeWithEndpoint(port, resourceRequest(port, token, bytes), timeoutMs)
  return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(answer))
}

async function managedResource(command, timeoutMs = ProtocolLimits.resourceTimeoutMs) {
  try {
    return await postResource(JSON.stringify(command), timeoutMs)
  } catch {
    throw new Error('Yon managed resource operation failed.')
  }
}

function gatewayAnswer(answer) {
  if (!isPlainObject(answer)) return false
  return Object.hasOwn(answer, 'result') || typeof answer.error === 'string'
}

async function callGateway(key, args) {
  let body
  try {
    body = JSON.stringify({ operation: 'gateway', key, arguments: args })
  } catch {
    return { error: 'invalid' }
  }
  if (typeof body !== 'string' || Buffer.byteLength(body) > ProtocolLimits.maxResourceBytes) return { error: 'invalid' }
  if (timeLeft() <= 0) return { error: 'timeout' }
  try {
    const answer = await postResource(body, timeLeft() + ProtocolLimits.gatewayAnswerMarginMs)
    if (gatewayAnswer(answer)) return answer
  } catch {
    if (timeLeft() <= 0) return { error: 'timeout' }
  }
  return { error: 'adapter' }
}

globalThis.YonGateway = Object.freeze({ call: callGateway })
globalThis.YonTopic = Object.freeze({ publish: (topic, payload) => managedResource({ operation: 'topic', topic, payload }) })

globalThis.DataSource = class DataSource {
  command(command) { return managedResource(command) }
  query(sql, parameters = []) { return managedResource({ operation: 'sql', sql, parameters, mode: 'rows' }) }
  execute(sql, parameters = []) {
    return managedResource(typeof sql === 'string' ? { operation: 'sql', sql, parameters, mode: 'execute' } : sql)
  }
  findById(collection, id) { return documentCommand('findById', { collection, id }) }
  insert(collection, document) { return documentCommand('insert', { collection, value: document }) }
  patch(collection, id, fields) { return documentCommand('patch', { collection, id, value: fields }) }
  deleteById(collection, id) { return documentCommand('deleteById', { collection, id }) }
  scroll(collection, where = null, cursor = null, size = 20) {
    return managedResource({ operation: 'document', command: 'scroll', collection, where, cursor, size }, ProtocolLimits.scrollTimeoutMs)
  }
  createCollection(name) { return documentCommand('createCollection', { collection: name }) }
  dropCollection(name) { return documentCommand('dropCollection', { collection: name }) }
  collectionExists(name) { return documentCommand('collectionExists', { collection: name }) }
  publish(topic, payload, key = null) { return queueCommand('publish', { topic, payload, key }) }
  claim(topic, group, maxMessages = 1, leaseMs = 30_000) { return queueCommand('claim', { topic, group, maxMessages, leaseMs }) }
  ack(topic, group, delivery) { return queueCommand('ack', { topic, group, id: delivery.id, receipt: delivery.receipt }) }
  nack(topic, group, delivery, delayMs = 0) {
    return queueCommand('nack', { topic, group, id: delivery.id, receipt: delivery.receipt, delayMs })
  }
}
function documentCommand(command, fields) { return managedResource({ operation: 'document', command, ...fields }) }
function queueCommand(command, fields) { return managedResource({ operation: 'queue', command, ...fields }) }
globalThis.RedisTemplate = class RedisTemplate {
  get(key) { return managedResource({ operation: 'redis', command: 'get', key }) }
  set(key, value, ttlSeconds) { return managedResource({ operation: 'redis', command: 'set', key, value, ttlSeconds }) }
  delete(key) { return managedResource({ operation: 'redis', command: 'delete', key }) }
}
function relayEnvironment(environment) {
  const kept = {}
  for (const [name, value] of Object.entries(environment)) {
    const upper = name.toUpperCase()
    if (upper === 'YON_APPLICATION_JSON' || upper === 'YON_HANDLER_WARM' || upper.startsWith('YON_RESOURCE_')) continue
    kept[name] = value
  }
  return Object.freeze(kept)
}

class MiddlewarePhases {
  static byOperation = new Map([
    ['middleware.before', 'before'],
    ['middleware.after', 'after'],
  ])
}

class RunnerState {
  static output = process.stdout
  static source = process.argv[2]
  static warm = process.env.YON_HANDLER_WARM === '1'
  static relayEnvironment = relayEnvironment(process.env)
  static buffered = Buffer.alloc(0)
  static activeRequest = null
  static deadline = 0
  static settled = false
  static cancelled = false
}

class WarmRequest {
  static useResources(envelope) {
    const resources = envelope.resources
    delete envelope.resources
    WarmRequest.forgetResources()
    if (!isPlainObject(resources)) return
    process.env.YON_RESOURCE_ENDPOINT = String(resources.endpoint ?? '')
    process.env.YON_RESOURCE_TOKEN = String(resources.token ?? '')
  }

  static forgetResources() {
    delete process.env.YON_RESOURCE_ENDPOINT
    delete process.env.YON_RESOURCE_TOKEN
  }

  static keepsRunning(exitCode) {
    return RunnerState.warm && exitCode === 0 && !RunnerState.cancelled
  }

  static finish() {
    RunnerState.activeRequest = null
    RunnerState.settled = false
    WarmRequest.forgetResources()
  }

  static announceReady() {
    const payload = Buffer.from(JSON.stringify({ protocol_version: 1, kind: 'ready' }), 'utf8')
    const prefix = Buffer.allocUnsafe(4)
    prefix.writeUInt32BE(payload.length)
    void writeOutput(Buffer.concat([prefix, payload])).catch(() => process.exit(74))
  }

  static endFrame(requestId) {
    const payload = Buffer.from(JSON.stringify({ protocol_version: 1, kind: 'end', request_id: requestId }), 'utf8')
    const prefix = Buffer.allocUnsafe(4)
    prefix.writeUInt32BE(payload.length)
    return Buffer.concat([prefix, payload])
  }
}

function sideband(...values) {
  const line = values.map((value) => {
    if (typeof value === 'string') return value
    try {
      return JSON.stringify(value)
    } catch {
      return String(value)
    }
  }).join(' ')
  process.stderr.write(`${line}\n`)
}

console.log = sideband
console.info = sideband
console.warn = sideband
console.error = sideband
console.debug = sideband

function publicMessage(error) {
  const value = error && typeof error.message === 'string'
    ? error.message
    : String(error ?? 'Handler failed.')
  return value.slice(0, 2048) || 'Handler failed.'
}

function writeOutput(chunk) {
  return new Promise((resolve, reject) => {
    let callbackDone = false
    let drainDone = false
    let settledWrite = false
    const cleanup = () => {
      RunnerState.output.off('error', onError)
      RunnerState.output.off('drain', onDrain)
    }
    const finish = () => {
      if (settledWrite || !callbackDone || !drainDone) return
      settledWrite = true
      cleanup()
      resolve()
    }
    const failWrite = (error) => {
      if (settledWrite) return
      settledWrite = true
      cleanup()
      reject(error)
    }
    const onError = (error) => failWrite(error)
    const onDrain = () => {
      drainDone = true
      finish()
    }
    RunnerState.output.once('error', onError)
    try {
      const writable = RunnerState.output.write(chunk, (error) => {
        if (error) {
          failWrite(error)
          return
        }
        callbackDone = true
        finish()
      })
      drainDone = writable
      if (!writable) RunnerState.output.once('drain', onDrain)
      finish()
    } catch (error) {
      failWrite(error)
    }
  })
}

function serializationFailure(message) {
  return Buffer.from(JSON.stringify({
    protocol_version: 1,
    kind: 'response',
    request_id: RunnerState.activeRequest?.request_id ?? 'adapter_error',
    status: 500,
    headers: {},
    error: { code: 'TY2203', message, retryable: false },
  }), 'utf8')
}

function writeFrame(envelope, exitCode = 0) {
  if (RunnerState.settled) return
  RunnerState.settled = true
  let payload
  try {
    payload = Buffer.from(JSON.stringify(envelope), 'utf8')
  } catch {
    payload = serializationFailure('Handler returned a value that is not JSON-serializable.')
  }
  if (payload.length > ProtocolLimits.maxFrameBytes) {
    payload = serializationFailure('Serialized handler result exceeds the protocol frame limit.')
  }
  const prefix = Buffer.allocUnsafe(4)
  prefix.writeUInt32BE(payload.length)
  const frame = Buffer.concat([prefix, payload])
  if (WarmRequest.keepsRunning(exitCode)) {
    WarmRequest.finish()
    void writeOutput(frame).catch(() => process.exit(74))
    return
  }
  void writeOutput(frame)
    .then(() => process.exit(exitCode), () => process.exit(74))
}

async function writeEvent(requestId, value) {
  let data
  try {
    data = JSON.stringify(value === undefined ? null : value)
  } catch {
    failure(requestId, 'TY2203', 'Streamed event is not JSON-serializable.')
    return false
  }
  const payload = Buffer.from(JSON.stringify({
    protocol_version: 1,
    kind: 'event',
    request_id: requestId,
    body: { encoding: 'utf8', data },
  }), 'utf8')
  if (payload.length > ProtocolLimits.maxFrameBytes) {
    failure(requestId, 'TY2203', 'Streamed event exceeds the protocol frame limit.')
    return false
  }
  const prefix = Buffer.allocUnsafe(4)
  prefix.writeUInt32BE(payload.length)
  try {
    await writeOutput(Buffer.concat([prefix, payload]))
  } catch {
    process.exit(74)
    return false
  }
  return true
}

function isStream(value) {
  return Boolean(value)
    && (typeof value[Symbol.asyncIterator] === 'function'
      || typeof value[Symbol.iterator] === 'function')
    && typeof value !== 'string'
    && !Array.isArray(value)
}

async function streamEvents(requestId, iterable) {
  try {
    for await (const event of iterable) {
      if (!await writeEvent(requestId, event)) return
    }
  } catch {
    failure(requestId, 'TY2204', 'The handler stream failed.')
    return
  }
  if (RunnerState.warm) {
    const frame = WarmRequest.endFrame(requestId)
    WarmRequest.finish()
    await writeOutput(frame).catch(() => process.exit(74))
    return
  }
  try {
    await writeOutput(Buffer.alloc(0))
    process.exit(0)
  } catch {
    process.exit(74)
  }
}

function failure(requestId, code, message, status = 500) {
  writeFrame({
    protocol_version: 1,
    kind: 'response',
    request_id: requestId,
    status,
    headers: {},
    error: {
      code,
      message: String(message).slice(0, 2048) || 'Handler failed.',
      retryable: false,
    },
  })
}

class CheckRejection {
  static status = 422
  static code = 'TY2205'
  static message = 'Input breaks a Checked rule.'
  static maxCauseDepth = 8

  static violations(error) {
    let cause = error
    for (let depth = 0; cause && depth < CheckRejection.maxCauseDepth; depth += 1) {
      if (cause.code === 'validation' && Array.isArray(cause.violations)) return cause.violations
      cause = cause.cause
    }
    return null
  }

  static answer(requestId, violations) {
    writeFrame({
      protocol_version: 1,
      kind: 'response',
      request_id: requestId,
      status: CheckRejection.status,
      headers: {},
      error: {
        code: CheckRejection.code,
        message: CheckRejection.message,
        retryable: false,
        violations: violations.map((entry) => ({ field: String(entry?.field), rule: String(entry?.rule) })),
      },
    })
  }
}

class YonHttp {
  static bodiless = new Set([101, 204, 205, 304])

  static request(frame) {
    const host = frame.headers?.host?.[0] ?? 'localhost'
    const url = new URL(frame.target ?? frame.route, `http://${host}`)
    const headers = new Headers()
    for (const [name, values] of Object.entries(frame.headers ?? {})) {
      for (const value of values) headers.append(name, value)
    }
    const init = { method: frame.method, headers }
    if (frame.body && frame.method !== 'GET' && frame.method !== 'HEAD') {
      init.body = frame.body.encoding === 'base64' ? Buffer.from(frame.body.data, 'base64') : frame.body.data
    }
    return new Request(url, init)
  }

  static response(status, headers, body) {
    const text = typeof body === 'string' ? body : JSON.stringify(body)
    const content = YonHttp.bodiless.has(status) ? null : text
    const list = new Headers()
    for (const [name, values] of Object.entries(headers ?? {})) {
      for (const value of Array.isArray(values) ? values : [values]) list.append(name, value)
    }
    return new Response(content, { status, headers: list })
  }

  static headers(headers) {
    const values = {}
    headers.forEach((value, name) => {
      if (name !== 'set-cookie') values[name] = [value]
    })
    const cookies = headers.getSetCookie?.() ?? []
    if (cookies.length) values['set-cookie'] = cookies
    return values
  }

  static async frame(response) {
    return {
      status: response.status,
      headers: YonHttp.headers(response.headers),
      body: { encoding: 'utf8', data: await response.text() },
    }
  }

  static async message(request) {
    const url = new URL(request.url)
    return {
      route: `${url.pathname}${url.search}`,
      method: request.method,
      headers: YonHttp.headers(request.headers),
      body: { encoding: 'utf8', data: await request.clone().text() },
    }
  }
}

globalThis.YonHttp = Object.freeze({ request: YonHttp.request })

function relayFailed(category) {
  sideband(JSON.stringify({
    event: 'handler.relay_failed',
    request_id: RunnerState.activeRequest?.request_id ?? 'unknown',
    category,
  }))
  return Response.json({ error: RelaySettings.failureMessage }, { status: 502 })
}

function timeLeft() {
  return Math.floor(RunnerState.deadline - performance.now())
}

function invocationDeadline(envelope) {
  const started = RunnerState.warm ? performance.now() : 0
  const milliseconds = Number(envelope.deadline_ms ?? RelaySettings.defaultDeadlineMs)
  if (!Number.isFinite(milliseconds)) return started + RelaySettings.defaultDeadlineMs
  return started + milliseconds
}

function collectOutput(stream, limit, capture) {
  return new Promise((resolve) => {
    const chunks = []
    let captured = 0
    let oversized = false
    stream.on('data', (chunk) => {
      const remaining = limit - captured
      if (remaining > 0 && capture) chunks.push(chunk.subarray(0, remaining))
      captured += Math.min(chunk.length, Math.max(remaining, 0))
      oversized ||= chunk.length > remaining
    })
    stream.on('end', () => resolve({ data: Buffer.concat(chunks), oversized }))
    stream.on('close', () => resolve({ data: Buffer.concat(chunks), oversized }))
    stream.on('error', () => resolve({ data: Buffer.concat(chunks), oversized: true }))
  })
}

function waitUntilDeadline(milliseconds) {
  let timer
  const expired = new Promise((resolve) => { timer = setTimeout(() => resolve(null), milliseconds) })
  return { expired, stop: () => clearTimeout(timer) }
}

function stopRelay(child) {
  child.kill('SIGKILL')
  child.stdout.destroy()
  child.stderr.destroy()
}

async function relayToComponent(command, request) {
  const [program, ...programArguments] = command ?? []
  if (!program) return relayFailed('start')
  if (timeLeft() <= 0) return relayFailed('timeout')
  let child
  try {
    child = spawn(program, programArguments, {
      stdio: ['pipe', 'pipe', 'pipe'],
      env: RunnerState.relayEnvironment,
    })
  } catch {
    return relayFailed('start')
  }
  const stdout = collectOutput(child.stdout, ProtocolLimits.maxRelayStdoutBytes, true)
  const stderr = collectOutput(child.stderr, ProtocolLimits.maxRelayStderrBytes, false)
  const exited = new Promise((resolve) => {
    child.once('error', () => resolve({ started: false, code: null }))
    child.once('close', (code) => resolve({ started: true, code }))
  })
  child.stdin.on('error', () => {})
  child.stdin.end(JSON.stringify(await YonHttp.message(request)))
  const deadline = waitUntilDeadline(Math.max(0, timeLeft()))
  const outcome = await Promise.race([Promise.all([exited, stdout, stderr]), deadline.expired])
  deadline.stop()
  if (outcome === null) {
    stopRelay(child)
    return relayFailed('timeout')
  }
  const [exit, stdoutResult, stderrResult] = outcome
  if (!exit.started) return relayFailed('start')
  if (stdoutResult.oversized || stderrResult.oversized) return relayFailed('overflow')
  if (exit.code !== 0) return relayFailed('exit')
  let envelope
  try {
    envelope = JSON.parse(stdoutResult.data.toString('utf8'))
  } catch {
    return relayFailed('protocol')
  }
  if (!isPlainObject(envelope)) return relayFailed('protocol')
  try {
    return YonHttp.response(envelope.status ?? 200, envelope.headers, envelope.body ?? '')
  } catch {
    return relayFailed('protocol')
  }
}

class SocketConnection {
  static ended = Symbol('socket ended')
  static current = null

  constructor(requestId) {
    this.requestId = requestId
    this.inbox = []
    this.waiting = null
    this.closed = false
    this.sessionAnswers = []
  }

  receive(envelope) {
    if (envelope.kind === 'sessions') {
      const answer = this.sessionAnswers.shift()
      if (answer) answer(Array.isArray(envelope.sessions) ? envelope.sessions : [])
      return
    }
    if (envelope.kind === 'close') this.closed = true
    else this.inbox.push(envelope)
    this.wake()
  }

  wake() {
    const waiting = this.waiting
    this.waiting = null
    if (waiting) waiting()
  }

  async next() {
    while (!this.inbox.length && !this.closed) {
      await new Promise((resolve) => { this.waiting = resolve })
    }
    const envelope = this.inbox.shift()
    if (!envelope) return SocketConnection.ended
    return JSON.parse(envelope.body.data)
  }

  async write(frame) {
    const payload = Buffer.from(JSON.stringify({ protocol_version: 1, request_id: this.requestId, ...frame }), 'utf8')
    if (payload.length > ProtocolLimits.maxFrameBytes) throw new Error('The socket message is too large.')
    const prefix = Buffer.allocUnsafe(4)
    prefix.writeUInt32BE(payload.length)
    await writeOutput(Buffer.concat([prefix, payload]))
  }

  send(message, to) {
    const data = JSON.stringify(message === undefined ? null : message)
    const frame = { kind: 'event', body: { encoding: 'utf8', data } }
    if (to !== undefined) frame.to = to
    return this.write(frame)
  }

  close(code) {
    this.closed = true
    this.wake()
    return this.write(code === undefined ? { kind: 'close' } : { kind: 'close', code })
  }

  openSessions() {
    return new Promise((resolve, reject) => {
      this.sessionAnswers.push((ids) => resolve(ids.map((id) => this.peer(String(id)))))
      this.write({ kind: 'sessions' }).catch(reject)
    })
  }

  peer(id) {
    return Object.freeze({ id, send: (message) => this.send(message, id) })
  }

  session() {
    return Object.freeze({
      id: this.requestId,
      send: (message) => this.send(message),
      close: () => this.close(),
      openSessions: () => this.openSessions(),
    })
  }
}

class YonSocket {
  static callMilliseconds = 30_000

  static async call(endpoint, name, values) {
    if (!name) return
    RunnerState.deadline = performance.now() + YonSocket.callMilliseconds
    await endpoint[name](...values)
  }

  static async attempt(endpoint, methods, session, name, values) {
    try {
      await YonSocket.call(endpoint, name, values)
    } catch (error) {
      sideband(JSON.stringify({ event: 'handler.socket_failed', request_id: session.id, message: publicMessage(error) }))
      try {
        await YonSocket.call(endpoint, methods.error, [session, error])
      } catch {}
    }
  }

  static async run(endpoint, methods, request) {
    const connection = SocketConnection.current ?? new SocketConnection(request.request_id)
    const session = connection.session()
    await YonSocket.attempt(endpoint, methods, session, methods.open, [session])
    for (;;) {
      const message = await connection.next()
      if (message === SocketConnection.ended) break
      await YonSocket.attempt(endpoint, methods, session, methods.message, [session, message])
    }
    try {
      await YonSocket.call(endpoint, methods.close, [session])
    } catch {}
    return SocketConnection.ended
  }
}

globalThis.YonSocket = Object.freeze({ run: YonSocket.run })

async function writeResult(request, result) {
  if (result instanceof Response) {
    writeFrame({
      protocol_version: 1,
      kind: 'response',
      request_id: request.request_id,
      ...await YonHttp.frame(result),
    })
    return
  }
  let data
  try {
    data = JSON.stringify(result === undefined ? null : result)
    if (typeof data !== 'string') throw new TypeError('Unsupported JSON value.')
  } catch {
    failure(request.request_id, 'TY2203', 'Handler returned a value that is not JSON-serializable.')
    return
  }
  writeFrame({
    protocol_version: 1,
    kind: 'response',
    request_id: request.request_id,
    status: 200,
    headers: { 'content-type': ['application/json; charset=utf-8'] },
    body: { encoding: 'utf8', data },
  })
}

async function invokeMiddleware(module, request, phase) {
  const Middleware = module.Middleware
  if (typeof Middleware !== 'function') {
    failure(request.request_id, 'TY2201', 'Module must export a class named Middleware.')
    return
  }
  const method = Middleware[phase]
  const result = typeof method === 'function'
    ? await method.call(Middleware, YonHttp.request(request))
    : undefined
  if (result !== undefined && result !== null && !(result instanceof Response)) {
    failure(request.request_id, 'TY2201', `Middleware ${phase}() must return a Fetch Response, or nothing to let the request pass.`)
    return
  }
  await writeResult(request, result ?? new Response(null, { status: 204 }))
}

async function invoke(request) {
  try {
    globalThis.relay ??= relayToComponent
    const module = await import(pathToFileURL(RunnerState.source).href)
    const phase = MiddlewarePhases.byOperation.get(request.operation)
    if (phase) {
      await invokeMiddleware(module, request, phase)
      return
    }
    const Handler = module.Handler
    if (typeof Handler !== 'function') {
      failure(request.request_id, 'TY2201', 'Module must export a class named Handler.')
      return
    }
    if (request.operation === 'event.deliver') {
      if (typeof Handler.__yonDeliver !== 'function') {
        failure(request.request_id, 'TY2202', 'Handler has no topic listeners.', 404)
        return
      }
      await Handler.__yonDeliver(JSON.parse(request.body?.data ?? '{}'))
      await writeResult(request, new Response(null, { status: 204 }))
      return
    }
    const method = Handler[request.method]
    if (typeof method !== 'function') {
      failure(request.request_id, 'TY2202', `Handler does not define static ${request.method}().`, 405)
      return
    }
    const argument = Handler.__yon_frames__ === true ? request : YonHttp.request(request)
    const result = await method.call(Handler, argument)
    if (result === SocketConnection.ended) {
      await writeOutput(Buffer.alloc(0))
      process.exit(0)
      return
    }
    if (isStream(result)) {
      await streamEvents(request.request_id, result)
      return
    }
    await writeResult(request, result)
  } catch (error) {
    const violations = CheckRejection.violations(error)
    if (violations) {
      CheckRejection.answer(RunnerState.activeRequest.request_id, violations)
      return
    }
    failure(RunnerState.activeRequest.request_id, 'TY2201', publicMessage(error))
  }
}

function accept(envelope) {
  if (!envelope || envelope.protocol_version !== 1) process.exit(70)
  if (envelope.kind === 'cancel') {
    if (RunnerState.activeRequest && envelope.request_id === RunnerState.activeRequest.request_id) {
      RunnerState.cancelled = true
      failure(envelope.request_id, 'TY2111', 'Handler invocation was cancelled.', 499)
    }
    return
  }
  const socket = SocketConnection.current
  if (socket && envelope.request_id === socket.requestId && ['message', 'close', 'sessions'].includes(envelope.kind)) {
    socket.receive(envelope)
    return
  }
  if (envelope.kind !== 'request' || RunnerState.activeRequest) process.exit(70)
  if (RunnerState.warm && envelope.operation === 'socket.open') process.exit(70)
  if (RunnerState.warm) WarmRequest.useResources(envelope)
  if (envelope.operation === 'socket.open') SocketConnection.current = new SocketConnection(envelope.request_id)
  RunnerState.activeRequest = envelope
  RunnerState.deadline = invocationDeadline(envelope)
  void invoke(envelope)
}

process.stdin.on('data', (chunk) => {
  RunnerState.buffered = Buffer.concat([RunnerState.buffered, chunk])
  if (RunnerState.buffered.length > ProtocolLimits.maxFrameBytes + 4) process.exit(70)
  while (RunnerState.buffered.length >= 4) {
    const length = RunnerState.buffered.readUInt32BE(0)
    if (length > ProtocolLimits.maxFrameBytes) process.exit(70)
    if (RunnerState.buffered.length < length + 4) return
    const payload = RunnerState.buffered.subarray(4, length + 4)
    RunnerState.buffered = RunnerState.buffered.subarray(length + 4)
    let envelope
    try {
      envelope = JSON.parse(payload.toString('utf8'))
    } catch {
      process.exit(70)
    }
    accept(envelope)
  }
})

process.stdin.on('error', () => process.exit(74))
process.stdin.on('end', () => {
  if (RunnerState.warm) process.exit(0)
})
if (RunnerState.warm) WarmRequest.announceReady()
process.stdin.resume()
