#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { chromium, firefox, webkit } from 'playwright'

class PasskeysBrowserCheck {
  static sources = ['tac-web.js', 'tac-passkeys.js'].map((name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url))

  static engines = [chromium, firefox, webkit]

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8">
    <meta name="tachyon-browser-apis" content="passkeys">
    <title>Passkeys</title></head><body>
    <button id="go">Go</button>
    <input id="name" autocomplete="username webauthn" aria-label="Name">
    <script src="/tac-web.js"></script>
    <script src="/tac-passkeys.js"></script>
    <script>
      window.sent = []
      TacWebApis.events = { publish: () => {}, send: (owner, name, value) => window.sent.push([owner, name, value]) }
      const web = globalThis[Symbol.for('tachyon.tac.web')]
      window.ask = (owner, call, args = []) => JSON.parse(web.calling(owner, () => web.host(JSON.stringify({ api: 'passkeys', call, args }))))
      window.next = null
      window.answered = null
      document.getElementById('go').addEventListener('click', () => { window.answered = window.next?.() })
    </script>
  </body></html>`

  static creation = (challenge, exclude = []) => JSON.stringify({
    rp: { id: 'localhost', name: 'Tachyon' },
    user: { id: 'dXNlci0x', name: 'ana@example.com', displayName: 'Ana' },
    challenge,
    pubKeyCredParams: [{ type: 'public-key', alg: -7 }, { type: 'public-key', alg: -257 }],
    authenticatorSelection: { residentKey: 'required', userVerification: 'required' },
    excludeCredentials: exclude.map((id) => ({ type: 'public-key', id })),
    timeout: 60000,
  })

  static request = (challenge, allow = [], timeout = 60000) => JSON.stringify({
    rpId: 'localhost',
    challenge,
    allowCredentials: allow.map((id) => ({ type: 'public-key', id })),
    userVerification: 'required',
    timeout,
  })

  static async run() {
    const files = Object.fromEntries(await Promise.all(PasskeysBrowserCheck.sources.map(async (source) => [`/${source.pathname.split('/').pop()}`, await readFile(source, 'utf8')])))
    const server = createServer((request, response) => {
      const script = files[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? PasskeysBrowserCheck.page)
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const url = `http://localhost:${server.address().port}/`
    try {
      for (const engine of PasskeysBrowserCheck.engines) await PasskeysBrowserCheck.runEngine(engine, url)
    } finally {
      server.close()
    }
  }

  static async runEngine(engine, url) {
    const browser = await engine.launch()
    try {
      const page = await browser.newPage()
      await page.goto(url)
      await PasskeysBrowserCheck.everyBrowser(page)
      if (engine.name() === 'chromium') await PasskeysBrowserCheck.ceremonies(page)
      console.log(`passkeys (${engine.name()}): ${engine.name() === 'chromium' ? 'create, get, exists, refused, cancelled, busy, cancel, the conditional offer and an offer stopped by a modal get with the virtual authenticator; ' : ''}supported, check, grant and user-action errors, invalid options and the JSON converter`)
    } finally {
      await browser.close()
    }
  }

  static async taken(page, count = 1) {
    await page.waitForFunction((wanted) => window.sent.length >= wanted, count, { timeout: 20000 })
    return page.evaluate(() => window.sent.splice(0))
  }

  static async forget(page) {
    await page.evaluate(() => window.ask('quiet', 'cancel'))
    await page.waitForTimeout(500)
    await page.evaluate(() => window.sent.splice(0))
  }

  static async clicked(page, owner, call, options) {
    await page.evaluate(([who, name, text]) => { window.next = () => window.ask(who, name, [text]) }, [owner, call, options])
    await page.click('#go')
    return page.evaluate(() => window.answered)
  }

  static async everyBrowser(page) {
    assert.deepEqual(await page.evaluate(() => window.ask('page', 'supported')), { value: true })
    assert.deepEqual(await page.evaluate(() => window.ask('page', 'check')), { value: null })
    const [[owner, name, support]] = await PasskeysBrowserCheck.taken(page)
    assert.deepEqual([owner, name, Object.keys(support).sort()], ['page', 'PasskeySupport', ['conditional', 'platform']])
    const [active, gesture] = await page.evaluate(([text]) => [navigator.userActivation.isActive, window.ask('quiet', 'create', [text])], [PasskeysBrowserCheck.creation('AQIDBAUGBwgJCgsMDQ4PEA')])
    if (!active) assert.match(gesture.error ?? '', /needs a user action/)
    if (active) await PasskeysBrowserCheck.forget(page)
    assert.deepEqual(await PasskeysBrowserCheck.clicked(page, 'bad', 'get', '{not json'), { value: null })
    assert.deepEqual(await PasskeysBrowserCheck.taken(page), [['bad', 'PasskeyFailed', { call: 'get', reason: 'invalid', message: 'The passkey options are not WebAuthn JSON of at most 65536 bytes. (SyntaxError)' }]])
    await PasskeysBrowserCheck.converterAgrees(page)
    const refused = await page.evaluate(() => {
      document.querySelector('meta[name="tachyon-browser-apis"]').content = 'share'
      const answers = [window.ask('page', 'supported'), window.ask('page', 'check')]
      document.querySelector('meta[name="tachyon-browser-apis"]').content = 'passkeys'
      return answers
    })
    assert.deepEqual(refused[0], { value: true })
    assert.match(refused[1].error ?? '', /The passkeys browser API is not granted/)
  }

  static async converterAgrees(page) {
    const decoded = await page.evaluate(([creation, request]) => {
      const listed = (options) => ({ challenge: [...new Uint8Array(options.challenge)], user: options.user ? [...new Uint8Array(options.user.id)] : [], ids: (options.excludeCredentials ?? options.allowCredentials ?? []).map((item) => [...new Uint8Array(item.id)]) })
      const own = [listed(TacPasskeys.decodedCreation(JSON.parse(creation))), listed(TacPasskeys.decodedRequest(JSON.parse(request)))]
      const browser = typeof PublicKeyCredential.parseCreationOptionsFromJSON === 'function'
        ? [listed(PublicKeyCredential.parseCreationOptionsFromJSON(JSON.parse(creation))), listed(PublicKeyCredential.parseRequestOptionsFromJSON(JSON.parse(request)))]
        : own
      const text = TacPasskeys.text(new Uint8Array([0, 251, 255, 62, 63]))
      return { own, browser, text, back: [...TacPasskeys.bytes(text)] }
    }, [PasskeysBrowserCheck.creation('AQIDBAUGBwgJCgsMDQ4PEA', ['-_8']), PasskeysBrowserCheck.request('EBEODw', ['AAEC'])])
    assert.deepEqual(decoded.own, decoded.browser)
    assert.deepEqual(decoded.own[0], { challenge: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], user: [117, 115, 101, 114, 45, 49], ids: [[251, 255]] })
    assert.equal(decoded.text, 'APv_Pj8')
    assert.deepEqual(decoded.back, [0, 251, 255, 62, 63])
  }

  static async ceremonies(page) {
    const client = await page.context().newCDPSession(page)
    await client.send('WebAuthn.enable')
    const { authenticatorId } = await client.send('WebAuthn.addVirtualAuthenticator', {
      options: { protocol: 'ctap2', transport: 'internal', hasResidentKey: true, hasUserVerification: true, isUserVerified: true, automaticPresenceSimulation: true },
    })
    await page.evaluate(() => window.ask('page', 'check'))
    assert.deepEqual(await PasskeysBrowserCheck.taken(page), [['page', 'PasskeySupport', { platform: true, conditional: true }]])
    const id = await PasskeysBrowserCheck.created(page)
    await PasskeysBrowserCheck.asserted(page, id)
    await PasskeysBrowserCheck.clicked(page, 'again', 'create', PasskeysBrowserCheck.creation('AgIDBAUGBwgJCgsMDQ4PEA', [id]))
    const [[, , excluded]] = await PasskeysBrowserCheck.taken(page)
    assert.deepEqual([excluded.call, excluded.reason], ['create', 'exists'])
    await PasskeysBrowserCheck.clicked(page, 'elsewhere', 'create', PasskeysBrowserCheck.creation('AwIDBAUGBwgJCgsMDQ4PEA').replace('"localhost"', '"example.com"'))
    const [[, , refused]] = await PasskeysBrowserCheck.taken(page)
    assert.deepEqual([refused.call, refused.reason, refused.message], ['create', 'refused', 'The relying party does not match this page. (SecurityError)'])
    console.log(`passkeys (chromium): ${await PasskeysBrowserCheck.offered(page, id)}`)
    await client.send('WebAuthn.setAutomaticPresenceSimulation', { authenticatorId, enabled: false })
    await PasskeysBrowserCheck.cancelled(page, id)
    await client.send('WebAuthn.removeVirtualAuthenticator', { authenticatorId })
  }

  static async created(page) {
    assert.deepEqual(await PasskeysBrowserCheck.clicked(page, 'maker', 'create', PasskeysBrowserCheck.creation('AQIDBAUGBwgJCgsMDQ4PEA')), { value: null })
    const [[owner, name, { credentialJson }]] = await PasskeysBrowserCheck.taken(page)
    assert.deepEqual([owner, name], ['maker', 'PasskeyCreated'])
    const credential = JSON.parse(credentialJson)
    assert.equal(credential.type, 'public-key')
    assert.equal(credential.id, credential.rawId)
    assert.equal(credential.authenticatorAttachment, 'platform')
    assert.ok(credential.response.attestationObject && credential.response.clientDataJSON)
    const client = JSON.parse(Buffer.from(credential.response.clientDataJSON, 'base64url').toString())
    assert.deepEqual([client.type, client.challenge], ['webauthn.create', 'AQIDBAUGBwgJCgsMDQ4PEA'])
    return credential.id
  }

  static async asserted(page, id) {
    await PasskeysBrowserCheck.clicked(page, 'signer', 'get', PasskeysBrowserCheck.request('BAUG', [id]))
    const [[owner, name, { credentialJson, conditional }]] = await PasskeysBrowserCheck.taken(page)
    assert.deepEqual([owner, name, conditional], ['signer', 'PasskeyAsserted', false])
    const assertion = JSON.parse(credentialJson)
    assert.equal(assertion.id, id)
    assert.equal(assertion.response.userHandle, 'dXNlci0x')
    assert.ok(assertion.response.signature && assertion.response.authenticatorData)
    const same = await page.evaluate(async ([text]) => {
      const canonical = (value) => JSON.stringify(value, (key, item) => item && typeof item === 'object' && !Array.isArray(item) ? Object.fromEntries(Object.entries(item).sort(([left], [right]) => left.localeCompare(right))) : item)
      const credential = await navigator.credentials.get({ publicKey: PublicKeyCredential.parseRequestOptionsFromJSON(JSON.parse(text)) })
      return [canonical(credential.toJSON()), canonical(JSON.parse(JSON.stringify(TacPasskeys.encoded(credential))))]
    }, [PasskeysBrowserCheck.request('BQYH', [id])])
    assert.equal(same[1], same[0], 'the own converter writes the same JSON as the browser')
  }

  static async offered(page, id) {
    assert.deepEqual(await page.evaluate(([text]) => window.ask('autofill', 'offer', [text]), [PasskeysBrowserCheck.request('CQoL')]), { value: null })
    const heard = await page.waitForFunction(() => window.sent.length > 0, null, { timeout: 3000 }).then(() => true, () => false)
    if (heard) {
      const [[owner, name, { credentialJson, conditional }]] = await PasskeysBrowserCheck.taken(page)
      assert.deepEqual([owner, name, conditional, JSON.parse(credentialJson).id], ['autofill', 'PasskeyAsserted', true, id])
      return 'the virtual authenticator answered the conditional offer'
    }
    await PasskeysBrowserCheck.clicked(page, 'modal', 'get', PasskeysBrowserCheck.request('DA0O', [id]))
    const sent = await PasskeysBrowserCheck.taken(page)
    assert.deepEqual(sent.map(([owner, name]) => [owner, name]), [['modal', 'PasskeyAsserted']], 'a modal get stops the pending offer, which sends nothing')
    return 'the conditional offer waited for autofill, and a modal get stopped it without an event'
  }

  static async cancelled(page, id) {
    assert.deepEqual(await page.evaluate(([text]) => window.ask('autofill', 'offer', [text]), [PasskeysBrowserCheck.request('Dg')]), { value: null })
    await page.waitForTimeout(300)
    await PasskeysBrowserCheck.clicked(page, 'slow', 'get', PasskeysBrowserCheck.request('Dw', [id], 1000))
    const timedOut = await PasskeysBrowserCheck.taken(page)
    assert.deepEqual(timedOut.map(([owner, name, value]) => [owner, name, value.call, value.reason]), [['slow', 'PasskeyFailed', 'get', 'cancelled']], 'the modal get stopped the pending offer, which sent nothing')
    await PasskeysBrowserCheck.clicked(page, 'waiting', 'get', PasskeysBrowserCheck.request('EA', [id]))
    await PasskeysBrowserCheck.clicked(page, 'second', 'get', PasskeysBrowserCheck.request('EQ', [id]))
    assert.deepEqual(await PasskeysBrowserCheck.taken(page), [['second', 'PasskeyFailed', { call: 'get', reason: 'busy', message: 'Another passkey prompt is already open.' }]])
    assert.deepEqual(await page.evaluate(() => window.ask('stopper', 'cancel')), { value: null })
    await page.waitForTimeout(500)
    assert.deepEqual(await page.evaluate(() => window.sent.splice(0)), [], 'a call that Tachyon cancels sends nothing')
  }
}

await PasskeysBrowserCheck.run()
