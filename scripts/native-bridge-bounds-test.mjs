import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { runInNewContext } from 'node:vm'

class NativeBridgeBoundsCheck {
  static source = new URL('../crates/tachyon-core/src/template/tac-client.js', import.meta.url)
  static limit = 65536

  static async run() {
    const source = await readFile(NativeBridgeBoundsCheck.source, 'utf8')
    const stateStart = source.indexOf('class NativeState {')
    const stateEnd = source.indexOf('class SharedData {', stateStart)
    const callStart = source.indexOf('const nativeCall = async ')
    const callEnd = source.indexOf('const nativeMember = ', callStart)
    assert.ok(stateStart > 0 && stateEnd > stateStart && callStart > 0 && callEnd > callStart)
    const environment = { ClientView: { plan: { route: '/' } }, TextEncoder, setTimeout, clearTimeout }
    runInNewContext(`${source.slice(stateStart, stateEnd)}\n${source.slice(callStart, callEnd)}\nglobalThis.call = nativeCall;`, environment)
    const request = { op: 'call', name: 'snapshot' }
    const service = 'SnapshotReaderService'
    let calls = 0
    let response = JSON.stringify({ ok: true, value: 'small' })
    environment.__tachyonNativeHostCall = () => { calls += 1; return response }
    assert.equal(await environment.call(request, 'owner', service), 'small')
    const overhead = Buffer.byteLength(JSON.stringify({ ok: true, value: '' }))
    const exact = 'x'.repeat(NativeBridgeBoundsCheck.limit - overhead)
    response = JSON.stringify({ ok: true, value: exact })
    assert.equal(Buffer.byteLength(response), NativeBridgeBoundsCheck.limit)
    assert.equal(await environment.call(request, 'owner', service), exact)
    response = JSON.stringify({ ok: true, value: exact + 'x' })
    await assert.rejects(environment.call(request, 'owner', service), /response exceeds 64 KiB/)
    response = JSON.stringify({ ok: true, value: '雪'.repeat(22000) })
    assert.ok(response.length < NativeBridgeBoundsCheck.limit)
    assert.ok(Buffer.byteLength(response) > NativeBridgeBoundsCheck.limit)
    await assert.rejects(environment.call(request, 'owner', service), /response exceeds 64 KiB/)
    response = JSON.stringify({ ok: true, value: 'accepted' })
    const base = { ...request, args: [''], route: '/', service, owner: 'owner' }
    const requestOverhead = Buffer.byteLength(JSON.stringify(base))
    const exactArgument = 'x'.repeat(NativeBridgeBoundsCheck.limit - requestOverhead)
    assert.equal(await environment.call({ ...request, args: [exactArgument] }, 'owner', service), 'accepted')
    const before = calls
    await assert.rejects(environment.call({ ...request, args: [exactArgument + 'x'] }, 'owner', service), /limit is 65536 bytes/)
    await assert.rejects(environment.call({ ...request, args: ['雪'.repeat(22000)] }, 'owner', service), /limit is 65536 bytes/)
    assert.equal(calls, before, 'oversized requests are refused before the host is invoked')
    assert.equal(await environment.call({ op: 'init', props: { value: 'x'.repeat(70000) } }, 'owner', service), 'accepted')
    await assert.rejects(environment.call({ op: 'init', props: { value: '雪'.repeat(350000) } }, 'owner', service), /limit is 1048576 bytes/)
    console.log('Native bridge UTF-8 bounds pass: exact 64KiB, byte overflow, multibyte requests/responses, no host invocation on refusal, distinct 1MiB startup limit.')
  }
}

await NativeBridgeBoundsCheck.run()
