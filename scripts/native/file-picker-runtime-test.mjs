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
  }
}

await FilePickerRuntimeCheck.run()
