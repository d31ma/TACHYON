import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { runInNewContext } from 'node:vm'

class FilePickerRuntimeCheck {
  static source = readFileSync(new URL('../../crates/tachyon-core/src/template/tac-native.js', import.meta.url), 'utf8')

  static async run() {
    const context = { AbortController, TacWebApis: { randomUuid: () => globalThis.crypto.randomUUID() } }
    runInNewContext(`${FilePickerRuntimeCheck.source}\nglobalThis.picker = TacNativeCapabilities.filePicker();`, context)
    assert.equal((await context.picker.open({})).status, 'unsupported')
    context.__tachyonNativeFilePicker = true
    assert.equal((await context.picker.open({})).status, 'denied')
    context.__tachyonNativeGrants = ['filePicker']
    const calls = []
    context.__tachyonNativeHostCall = async (name, payload) => {
      calls.push([name, JSON.parse(payload)])
      return JSON.stringify({ status: 'cancelled' })
    }
    assert.equal((await context.picker.open({})).status, 'cancelled')
    assert.equal(calls[0][0], 'filePicker.open')
    assert.equal((await context.picker.open({ multiple: true })).status, 'invalid')
    const cancellation = new AbortController()
    const stopped = runInNewContext(`${FilePickerRuntimeCheck.source}\nTacNativeCapabilities.filePicker(globalThis.signal)`, { ...context, signal: cancellation.signal })
    cancellation.abort()
    assert.equal((await stopped.open({})).status, 'disposed')
    await Promise.resolve()
    assert.equal(calls.at(-1)[0], 'filePicker.dispose')
    context.__tachyonNativeHostCall = async () => { throw new Error('sensitive path') }
    assert.equal((await context.picker.open({})).status, 'unavailable')
    console.log('native FilePicker provider: unsupported, denied, cancellation, options, disposal and transport failure passed')
    FilePickerRuntimeCheck.runNow()
  }

  static runNow() {
    const sent = []
    let answer = { status: 200, responseText: JSON.stringify({ status: 'selected', handle: 'h1', name: 'notes.txt' }) }
    class XMLHttpRequest {
      open(method, url, asynchronous) { this.call = { method, url, asynchronous, headers: {} } }
      setRequestHeader(name, value) { this.call.headers[name] = value }
      send(body) { sent.push(JSON.parse(JSON.stringify({ ...this.call, body: JSON.parse(body) }))); Object.assign(this, answer) }
    }
    let uuid = 0
    const context = { XMLHttpRequest, TacWebApis: { randomUuid: () => `00000000-0000-4000-8000-00000000000${++uuid}` } }
    runInNewContext(`${FilePickerRuntimeCheck.source}\nglobalThis.nowText = (...values) => TacNativeCapabilities.now(...values)`, context)
    context.now = (...values) => JSON.parse(context.nowText(...values))
    const owner = JSON.stringify({ owner: 'shared-1', types: ['text/plain'] })
    assert.equal(context.now('swift', 'filePicker.open', owner).status, 'unsupported')
    context.__tachyonNativeFilePickerRoute = true
    assert.equal(context.now('swift', 'filePicker.open', owner).status, 'denied')
    context.__tachyonNativeGrants = ['filePicker']
    assert.equal(context.now('swift', 'filePicker.available', owner).status, 'available')
    assert.equal(sent.length, 0)
    assert.deepEqual(context.now('swift', 'filePicker.open', owner), { status: 'selected', handle: 'h1', name: 'notes.txt' })
    assert.deepEqual(sent[0], { method: 'POST', url: '/__tachyon/capability/filePicker.open', asynchronous: false, headers: { 'X-Tachyon-Capability': 'filePicker.open', 'X-Tachyon-Capability-Request': encodeURIComponent(JSON.stringify({ owner: '00000000-0000-4000-8000-000000000001', types: ['text/plain'] })) }, body: { owner: '00000000-0000-4000-8000-000000000001', types: ['text/plain'] } })
    context.now('kotlin', 'filePicker.open', owner)
    assert.equal(sent[1].body.owner, '00000000-0000-4000-8000-000000000002')
    context.now('swift', 'filePicker.readText', JSON.stringify({ owner: 'shared-1', handle: 'h1' }))
    assert.equal(sent[2].body.owner, '00000000-0000-4000-8000-000000000001')
    assert.equal(context.now('swift', 'filePicker.delete', owner).status, 'invalid')
    assert.equal(context.now('swift', 'filePicker.open', '{').status, 'invalid')
    assert.equal(context.now('csharp', 'filePicker.dispose', JSON.stringify({ owner: 'never-used' })).status, 'disposed')
    assert.equal(sent.length, 3)
    answer = { status: 200, responseText: JSON.stringify({ status: 'disposed' }) }
    assert.equal(context.now('swift', 'filePicker.dispose', owner).status, 'disposed')
    assert.deepEqual([sent[3].url, sent[3].body], ['/__tachyon/capability/filePicker.dispose', { owner: '00000000-0000-4000-8000-000000000001' }])
    answer = { status: 200, responseText: JSON.stringify({ status: 'selected' }) }
    context.now('swift', 'filePicker.open', owner)
    assert.equal(sent[4].body.owner, '00000000-0000-4000-8000-000000000003')
    answer = { status: 404, responseText: 'Not Found' }
    assert.equal(context.now('swift', 'filePicker.open', owner).status, 'unavailable')
    answer = { status: 200, responseText: JSON.stringify({ status: 'C:\\secret' }) }
    assert.equal(context.now('swift', 'filePicker.open', owner).status, 'unavailable')
    console.log('native FilePicker page route: unsupported, denied, available, owner mapping per language, disposal, bad operations and bad answers passed')
  }
}

await FilePickerRuntimeCheck.run()
