import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { runInNewContext } from 'node:vm'

class BrowserHttpRuntimeCheck {
  static source = new URL('../crates/tachyon-core/src/template/tac-http.js', import.meta.url)

  static request(values = {}) {
    return JSON.stringify({ method: 'GET', path: '/api/greeting', query: {}, headers: {}, redirect: 'error',
      credentials: 'omit', deadline: Date.now() + 1000, maxResponseBytes: 1048576, ...values })
  }

  static streamed(chunks, cancelledReads) {
    const encoded = chunks.map(chunk => new TextEncoder().encode(chunk))
    const reader = {
      read: async () => encoded.length > 0 ? { done: false, value: encoded.shift() } : { done: true, value: undefined },
      cancel: async () => { cancelledReads.push(encoded.length) },
      releaseLock: () => {}
    }
    return { ok: true, body: { getReader: () => reader } }
  }

  static async run() {
    const calls = []
    const environment = { AbortController, Headers, URL, TextEncoder, TextDecoder, setTimeout, clearTimeout,
      location: { href: 'https://app.example/page', origin: 'https://app.example' },
      fetch: async (url, options) => { calls.push({ url, options }); return new Response('{"message":"Hello from Yon"}') } }
    runInNewContext(`${await readFile(BrowserHttpRuntimeCheck.source, 'utf8')}\nglobalThis.http = TacBrowserHttp;`, environment)
    const answer = async (request) => JSON.parse(await environment.http.start(request).answer)
    assert.equal((await answer(BrowserHttpRuntimeCheck.request())).value.message, 'Hello from Yon')
    assert.equal(calls[0].options.credentials, 'omit')
    assert.equal(calls[0].options.redirect, 'error')
    assert.equal(calls[0].options.mode, 'same-origin')
    assert.equal(calls[0].options.cache, 'no-store')
    await answer(BrowserHttpRuntimeCheck.request({ publicRead: true, query: { page: 2, word: 'a b' } }))
    assert.equal(calls.at(-1).options.cache, 'default')
    assert.equal(calls.at(-1).url, 'https://app.example/api/greeting?page=2&word=a+b')
    await answer(BrowserHttpRuntimeCheck.request({ publicRead: true, revalidate: true }))
    assert.equal(calls.at(-1).options.cache, 'no-cache')
    await answer(BrowserHttpRuntimeCheck.request({ publicRead: true, headers: { Authorization: 'Bearer explicit' } }))
    assert.equal(calls.at(-1).options.cache, 'no-store', 'authenticated reads cannot use the public cache')
    await answer(BrowserHttpRuntimeCheck.request({ method: 'POST', body: { message: 'hello' }, headers: { 'X-Request': 'safe' } }))
    assert.equal(calls.at(-1).options.body, '{"message":"hello"}')
    assert.equal(calls.at(-1).options.headers.get('content-type'), 'application/json')
    const before = calls.length
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ path: '//foreign.example/private' }))).error.code, 'HTTP_ORIGIN')
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ headers: { Cookie: 'secret' } }))).error.code, 'HTTP_HEADER')
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ credentials: 'include' }))).error.code, 'HTTP_REQUEST')
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ body: {} }))).error.code, 'HTTP_BODY')
    assert.equal(calls.length, before)
    environment.fetch = async () => new Response('private failure', { status: 403 })
    const refused = await answer(BrowserHttpRuntimeCheck.request())
    assert.deepEqual(refused.error, { code: 'HTTP_STATUS', status: 403 })
    assert.ok(!JSON.stringify(refused).includes('private'), 'the answer body stays out')
    environment.fetch = async (url, options) => { calls.push({ url, options }); return new Response('{}') }
    const large = 'x'.repeat(900000)
    assert.deepEqual((await answer(BrowserHttpRuntimeCheck.request({ method: 'POST', body: { large } }))).value, {})
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ method: 'POST', body: { large: large + large } }))).error.code, 'HTTP_REQUEST')
    environment.fetch = async () => { throw new Error('private host path and token') }
    const failure = await answer(BrowserHttpRuntimeCheck.request())
    assert.equal(failure.error.code, 'HTTP_NETWORK')
    assert.ok(!JSON.stringify(failure).includes('private'))
    environment.fetch = async () => new Response('12345')
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ maxResponseBytes: 4 }))).error.code, 'HTTP_LIMIT')
    const cancelledReads = []
    environment.fetch = async () => BrowserHttpRuntimeCheck.streamed(['{"message":', '"whole"}'], cancelledReads)
    assert.equal((await answer(BrowserHttpRuntimeCheck.request())).value.message, 'whole')
    assert.deepEqual(cancelledReads, [], 'a fully read body is not cancelled, because Chromium then reports the finished request as aborted')
    environment.fetch = async () => BrowserHttpRuntimeCheck.streamed(['123', '45'], cancelledReads)
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ maxResponseBytes: 4 }))).error.code, 'HTTP_LIMIT')
    assert.equal(cancelledReads.length, 1, 'a body over the limit is cancelled')
    const declared = (text, length) => {
      const response = new Response(text, { headers: { 'content-length': String(length) } })
      Object.defineProperty(response, 'body', { value: { getReader: () => { throw new Error('a declared body is read whole') } } })
      return response
    }
    environment.fetch = async () => declared('{"message":"declared"}', 22)
    assert.equal((await answer(BrowserHttpRuntimeCheck.request())).value.message, 'declared', 'a body with a declared length is read whole, so Chromium finishes the load before the answer')
    environment.fetch = async () => declared('12345', 5)
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ maxResponseBytes: 4 }))).error.code, 'HTTP_LIMIT', 'a declared length over the limit is refused before reading')
    environment.fetch = async () => new Response('not JSON')
    assert.equal((await answer(BrowserHttpRuntimeCheck.request())).error.code, 'HTTP_RESPONSE')
    environment.fetch = async () => new Response(new Uint8Array([0xff]))
    assert.equal((await answer(BrowserHttpRuntimeCheck.request())).error.code, 'HTTP_RESPONSE')
    environment.fetch = async (_, options) => new Promise((_, reject) => {
      if (options.signal.aborted) reject(new Error('stopped'))
      else options.signal.addEventListener('abort', () => reject(new Error('stopped')), { once: true })
    })
    const cancelled = environment.http.start(BrowserHttpRuntimeCheck.request())
    cancelled.cancel()
    assert.equal(JSON.parse(await cancelled.answer).error.code, 'CANCELLED')
    assert.equal((await answer(BrowserHttpRuntimeCheck.request({ deadline: Date.now() + 15 }))).error.code, 'DEADLINE')
    assert.equal(environment.http.pending, 0)
    const waiting = Array.from({ length: 64 }, () => environment.http.start(BrowserHttpRuntimeCheck.request()))
    assert.equal((await answer(BrowserHttpRuntimeCheck.request())).error.code, 'HTTP_CAPACITY')
    waiting.forEach(ticket => ticket.cancel())
    await Promise.all(waiting.map(ticket => ticket.answer))
    assert.equal(environment.http.pending, 0)
    console.log('browser HTTP host: async answer, same-origin, credentials/cache policy, query/body/header rules, limits, cancellation, deadlines and sanitized errors pass')
  }
}

await BrowserHttpRuntimeCheck.run()
