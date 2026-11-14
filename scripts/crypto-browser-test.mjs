#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { createServer } from 'node:http'
import { hkdfSync, pbkdf2Sync } from 'node:crypto'
import { chromium, firefox, webkit } from 'playwright'

class CryptoBrowserCheck {
  static sources = ['tac-web.js', 'tac-crypto.js'].map((name) => new URL(`../crates/tachyon-core/src/template/${name}`, import.meta.url))

  static engines = [chromium, firefox, webkit]

  static insecureHost = 'insecure.test'

  static page = `<!doctype html><html lang="en"><head><meta charset="utf-8">
    <meta name="tachyon-browser-apis" content="crypto">
    <title>Crypto</title></head><body>
    <script src="/tac-web.js"></script>
    <script src="/tac-crypto.js"></script>
    <script>
      window.sent = []
      TacWebApis.events = { publish: () => {}, send: (owner, name, value) => window.sent.push([owner, name, value]) }
      const web = globalThis[Symbol.for('tachyon.tac.web')]
      window.ask = (call, args = []) => JSON.parse(web.calling('page', () => web.host(JSON.stringify({ api: 'crypto', call, args }))))
      window.answer = async (call, args = []) => {
        const asked = window.ask(call, args)
        if (!asked.value || call === 'supported') return asked
        for (;;) {
          const index = window.sent.findIndex((item) => item[2].request === asked.value)
          if (index >= 0) return window.sent.splice(index, 1)[0]
          await new Promise((resolve) => setTimeout(resolve, 5))
        }
      }
      window.stored = () => new Promise((resolve, reject) => {
        const opened = indexedDB.open('tachyon-keys', 1)
        opened.onerror = () => reject(opened.error)
        opened.onsuccess = () => {
          const request = opened.result.transaction('keys', 'readonly').objectStore('keys').getAll()
          request.onsuccess = () => { opened.result.close(); resolve(request.result) }
          request.onerror = () => reject(request.error)
        }
      })
    </script>
  </body></html>`

  static hex = (text) => Buffer.from(text, 'hex').toString('base64url')

  static filled = (size, value) => Buffer.alloc(size, value).toString('base64url')

  static async run() {
    const files = Object.fromEntries(await Promise.all(CryptoBrowserCheck.sources.map(async (source) => [`/${source.pathname.split('/').pop()}`, await readFile(source, 'utf8')])))
    const server = createServer((request, response) => {
      const script = files[request.url]
      response.writeHead(200, { 'content-type': script ? 'text/javascript' : 'text/html' })
      response.end(script ?? CryptoBrowserCheck.page)
    })
    await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve))
    const port = server.address().port
    try {
      for (const engine of CryptoBrowserCheck.engines) await CryptoBrowserCheck.runEngine(engine, port)
    } finally {
      server.close()
    }
  }

  static async runEngine(engine, port) {
    const chromiumOnly = engine.name() === 'chromium'
    const args = chromiumOnly ? [`--host-resolver-rules=MAP ${CryptoBrowserCheck.insecureHost} 127.0.0.1`] : []
    const browser = await engine.launch({ args })
    try {
      const context = await browser.newContext()
      const page = await context.newPage()
      await page.goto(`http://localhost:${port}/`)
      await CryptoBrowserCheck.supported(page)
      await CryptoBrowserCheck.knownAnswers(page)
      await CryptoBrowserCheck.roundTrips(page)
      await CryptoBrowserCheck.keptKeys(page)
      if (chromiumOnly) await CryptoBrowserCheck.insecure(context, port)
      console.log(`crypto (${engine.name()}): check, RFC 8032 verify (test 2), RFC 5869 HKDF, SHA-256, NIST AES-256-GCM, PBKDF2-SHA-256 at 600,000 rounds, X25519 agreement both ways, a small-order key refused, sign and verify, 64 unique nonces, a 32,768-byte message, a kept key across a reload that cannot be exported, forget and forgetAll${chromiumOnly ? ', and unsupported on a plain http page' : ''}`)
    } finally {
      await browser.close()
    }
  }

  static async ask(page, call, args = []) {
    const [owner, name, value] = await page.evaluate(([what, values]) => window.answer(what, values), [call, args])
    assert.equal(owner, 'page')
    assert.notEqual(name, 'CryptoFailed', `${call}: ${JSON.stringify(value)}`)
    return value
  }

  static async refused(page, call, args) {
    const [, name, value] = await page.evaluate(([what, values]) => window.answer(what, values), [call, args])
    assert.equal(name, 'CryptoFailed', `${call} should fail`)
    return value.reason
  }

  static async supported(page) {
    assert.deepEqual(await page.evaluate(() => window.ask('supported')), { value: true })
    const support = await CryptoBrowserCheck.ask(page, 'check')
    assert.deepEqual({ ...support, request: '' }, { request: '', x25519: true, ed25519: true, aesGcm: true, hkdf: true, pbkdf2: true, keyStore: 'browser' })
  }

  static async knownAnswers(page) {
    const { hex } = CryptoBrowserCheck
    const rfc8032 = await CryptoBrowserCheck.ask(page, 'verify', [hex('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c'), hex('72'), hex('92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00')])
    assert.equal(rfc8032.valid, true, 'RFC 8032 test 2')
    const hkdf = await CryptoBrowserCheck.ask(page, 'hkdf', [CryptoBrowserCheck.filled(22, 11), hex('000102030405060708090a0b0c'), hex('f0f1f2f3f4f5f6f7f8f9'), 42])
    assert.equal(Buffer.from(hkdf.bytes, 'base64url').toString('hex'), '3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865', 'RFC 5869 test 1')
    const digest = await CryptoBrowserCheck.ask(page, 'hash', [Buffer.from('abc').toString('base64url')])
    assert.equal(Buffer.from(digest.digest, 'base64url').toString('hex'), 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')
    await CryptoBrowserCheck.ask(page, 'importKey', ['nist', hex('feffe9928665731c6d6a8f9467308308feffe9928665731c6d6a8f9467308308'), 'encrypt', false])
    const sealed = hex('01cafebabefacedbaddecaf888522dc1f099567d07f47f37a32a84427d643a8cdcbfe5c0c97598a2bd2555d1aa8cb08e48590dbb3da7b08b1056828838c5f61e6393ba7a0abcc9f66276fc6ece0f4e1768cddf8853bb2d551b')
    const opened = await CryptoBrowserCheck.ask(page, 'decrypt', ['nist', sealed, hex('feedfacedeadbeeffeedfacedeadbeefabaddad2')])
    assert.equal(Buffer.from(opened.data, 'base64url').toString('hex'), 'd9313225f88406e5a55909c5aff5269a86a7a9531534f7da2e4c303d8a318a721c3c0c95956809532fcf0e2449a6b525b16aedf5aa0de657ba637b39', 'NIST GCM test 16')
    assert.equal(await CryptoBrowserCheck.refused(page, 'decrypt', ['nist', sealed, '']), 'rejected')
    const salt = CryptoBrowserCheck.filled(16, 4)
    await CryptoBrowserCheck.ask(page, 'fromCode', ['code', 'correct horse battery staple', salt, 'encrypt', false])
    await CryptoBrowserCheck.ask(page, 'importKey', ['code-expected', pbkdf2Sync('correct horse battery staple', Buffer.alloc(16, 4), 600000, 32, 'sha256').toString('base64url'), 'encrypt', false])
    const fromCode = await CryptoBrowserCheck.ask(page, 'encrypt', ['code', 'Bg', ''])
    assert.equal((await CryptoBrowserCheck.ask(page, 'decrypt', ['code-expected', fromCode.sealed, ''])).data, 'Bg', 'PBKDF2-SHA-256 at 600,000 rounds')
    await CryptoBrowserCheck.ask(page, 'importKey', ['root', CryptoBrowserCheck.filled(32, 3), 'derive', false])
    await CryptoBrowserCheck.ask(page, 'deriveKey', ['root', 'BQ', 'Bg', 'from-root', 'encrypt', false])
    await CryptoBrowserCheck.ask(page, 'importKey', ['root-expected', Buffer.from(hkdfSync('sha256', Buffer.alloc(32, 3), Buffer.from([5]), Buffer.from([6]), 32)).toString('base64url'), 'encrypt', false])
    const fromRoot = await CryptoBrowserCheck.ask(page, 'encrypt', ['from-root', 'CA', ''])
    assert.equal((await CryptoBrowserCheck.ask(page, 'decrypt', ['root-expected', fromRoot.sealed, ''])).data, 'CA', 'deriveKey is HKDF-SHA-256')
  }

  static async roundTrips(page) {
    const alice = await CryptoBrowserCheck.ask(page, 'makeKeyPair', ['alice', 'x25519', false])
    const bob = await CryptoBrowserCheck.ask(page, 'makeKeyPair', ['bob', 'x25519', false])
    await CryptoBrowserCheck.ask(page, 'agree', ['alice', bob.publicKey, 'AQ', 'Ag', 'alice-bob', 'encrypt', false])
    await CryptoBrowserCheck.ask(page, 'agree', ['bob', alice.publicKey, 'AQ', 'Ag', 'bob-alice', 'encrypt', false])
    const between = await CryptoBrowserCheck.ask(page, 'encrypt', ['alice-bob', 'BAI', 'CQ'])
    assert.equal((await CryptoBrowserCheck.ask(page, 'decrypt', ['bob-alice', between.sealed, 'CQ'])).data, 'BAI')
    assert.equal(await CryptoBrowserCheck.refused(page, 'agree', ['alice', CryptoBrowserCheck.filled(32, 0), '', '', 'zero', 'derive', false]), 'invalid')
    const signer = await CryptoBrowserCheck.ask(page, 'makeKeyPair', ['signer', 'ed25519', false])
    const signed = await CryptoBrowserCheck.ask(page, 'sign', ['signer', 'AQID'])
    assert.equal((await CryptoBrowserCheck.ask(page, 'verify', [signer.publicKey, 'AQID', signed.signature])).valid, true)
    assert.equal((await CryptoBrowserCheck.ask(page, 'verify', [signer.publicKey, 'AQIE', signed.signature])).valid, false)
    const nonces = new Set()
    for (let round = 0; round < 64; round += 1) {
      const { sealed } = await CryptoBrowserCheck.ask(page, 'encrypt', ['alice-bob', 'AQ', ''])
      nonces.add(Buffer.from(sealed, 'base64url').subarray(1, 13).toString('hex'))
    }
    assert.equal(nonces.size, 64, 'every nonce is new')
    const largest = await CryptoBrowserCheck.ask(page, 'encrypt', ['alice-bob', CryptoBrowserCheck.filled(32768, 7), ''])
    assert.equal(Buffer.from((await CryptoBrowserCheck.ask(page, 'decrypt', ['bob-alice', largest.sealed, ''])).data, 'base64url').length, 32768)
    const usages = await page.evaluate(async () => (await globalThis[Symbol.for('tachyon.tac.crypto.hooks')].key('root'))?.usages.slice().sort().join(' '))
    assert.equal(usages, 'deriveBits deriveKey')
    assert.equal(await page.evaluate(() => globalThis[Symbol.for('tachyon.tac.crypto.hooks')].key('signer')), null)
  }

  static async keptKeys(page) {
    await CryptoBrowserCheck.ask(page, 'forgetAll')
    const made = await CryptoBrowserCheck.ask(page, 'makeKeyPair', ['device', 'ed25519', true])
    await CryptoBrowserCheck.ask(page, 'importKey', ['backup', CryptoBrowserCheck.filled(32, 9), 'encrypt', true])
    const sealed = await CryptoBrowserCheck.ask(page, 'encrypt', ['backup', 'AQ', ''])
    await page.reload()
    assert.equal((await CryptoBrowserCheck.ask(page, 'publicKey', ['device'])).publicKey, made.publicKey, 'a kept key survives a reload')
    const signed = await CryptoBrowserCheck.ask(page, 'sign', ['device', 'AQ'])
    assert.equal((await CryptoBrowserCheck.ask(page, 'verify', [made.publicKey, 'AQ', signed.signature])).valid, true)
    assert.equal((await CryptoBrowserCheck.ask(page, 'decrypt', ['backup', sealed.sealed, ''])).data, 'AQ')
    const exported = await page.evaluate(async () => Promise.all((await window.stored()).map((record) => crypto.subtle.exportKey(record.key.type === 'private' ? 'pkcs8' : 'raw', record.key).then(() => 'exported', (error) => `${record.name} ${record.key.extractable} ${error.name}`))))
    assert.deepEqual(exported.sort(), ['backup false InvalidAccessError', 'device false InvalidAccessError'], 'a stored key cannot be exported')
    assert.equal(await CryptoBrowserCheck.refused(page, 'makeKeyPair', ['device', 'x25519', true]), 'exists')
    await CryptoBrowserCheck.ask(page, 'forget', ['device'])
    assert.equal(await CryptoBrowserCheck.refused(page, 'publicKey', ['device']), 'unknown')
    await CryptoBrowserCheck.ask(page, 'forgetAll')
    assert.equal((await page.evaluate(() => window.stored())).length, 0, 'forgetAll empties the key store')
  }

  static async insecure(context, port) {
    const page = await context.newPage()
    await page.goto(`http://${CryptoBrowserCheck.insecureHost}:${port}/`)
    assert.equal(await page.evaluate(() => window.isSecureContext), false)
    assert.deepEqual(await page.evaluate(() => window.ask('supported')), { value: false })
    const [, , support] = await page.evaluate(() => window.answer('check'))
    assert.deepEqual({ ...support, request: '' }, { request: '', x25519: false, ed25519: false, aesGcm: false, hkdf: false, pbkdf2: false, keyStore: 'none' })
    assert.equal(await CryptoBrowserCheck.refused(page, 'makeKeyPair', ['device', 'ed25519', false]), 'unsupported')
    await page.close()
  }
}

await CryptoBrowserCheck.run()
