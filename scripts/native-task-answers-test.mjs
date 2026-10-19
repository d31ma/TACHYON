import assert from 'node:assert/strict'
import { readFile } from 'node:fs/promises'
import { runInNewContext } from 'node:vm'

class NativeTaskAnswersCheck {
  static source = new URL('../crates/tachyon-core/src/template/tac-client.js', import.meta.url)
  static owner = '00000000-0000-4000-8000-000000000071'
  static service = 'NoteService'

  static async run() {
    const environment = await NativeTaskAnswersCheck.environment()
    await NativeTaskAnswersCheck.taskAnswerWaitsForItsSignal(environment)
    await NativeTaskAnswersCheck.earlySignalIsKept(environment)
    await NativeTaskAnswersCheck.failedTaskRejects(environment)
    await NativeTaskAnswersCheck.disposedOwnerRejects(environment)
    await NativeTaskAnswersCheck.otherOwnersAreIgnored(environment)
    await NativeTaskAnswersCheck.synchronousAnswersStayTheSame(environment)
    console.log('Native task answers pass: task signal, early signal, failed task, disposed owner, foreign owner, synchronous answers.')
  }

  static async environment() {
    const source = await readFile(NativeTaskAnswersCheck.source, 'utf8')
    const stateStart = source.indexOf('class NativeState {')
    const stateEnd = source.indexOf('class SharedData {', stateStart)
    const callStart = source.indexOf('const nativeCall = async ')
    const callEnd = source.indexOf('const nativeMember = ', callStart)
    assert.ok(stateStart > 0 && stateEnd > stateStart && callStart > 0 && callEnd > callStart)
    const environment = { ClientView: { plan: { route: '/' } }, TextEncoder, setTimeout, clearTimeout, JSON }
    runInNewContext(`${source.slice(stateStart, stateEnd)}\n${source.slice(callStart, callEnd)}\nglobalThis.call = nativeCall;\nglobalThis.tasks = NativeTasks;`, environment)
    return environment
  }

  static signal(task, answer, owner = NativeTaskAnswersCheck.owner) {
    return { route: '/', service: NativeTaskAnswersCheck.service, owner, task, answer: JSON.stringify(answer) }
  }

  static call(environment) {
    return environment.call({ op: 'call', name: 'load', args: [] }, NativeTaskAnswersCheck.owner, NativeTaskAnswersCheck.service)
  }

  static async taskAnswerWaitsForItsSignal(environment) {
    environment.__tachyonNativeHostCall = () => JSON.stringify({ task: 1 })
    const pending = NativeTaskAnswersCheck.call(environment)
    await new Promise((resolve) => setTimeout(resolve, 10))
    environment.tasks.done(NativeTaskAnswersCheck.signal(1, { value: { text: 'from Yon' } }))
    assert.deepEqual(await pending, { text: 'from Yon' })
  }

  static async earlySignalIsKept(environment) {
    environment.__tachyonNativeHostCall = () => {
      environment.tasks.done(NativeTaskAnswersCheck.signal(2, { value: 'early' }))
      return JSON.stringify({ task: 2 })
    }
    assert.equal(await NativeTaskAnswersCheck.call(environment), 'early')
  }

  static async failedTaskRejects(environment) {
    environment.__tachyonNativeHostCall = () => JSON.stringify({ task: 3 })
    const pending = NativeTaskAnswersCheck.call(environment)
    await new Promise((resolve) => setTimeout(resolve, 10))
    environment.tasks.done(NativeTaskAnswersCheck.signal(3, { error: 'Web service task failed.', code: 'HTTP_STATUS' }))
    await assert.rejects(pending, /Tac native call failed/)
  }

  static async disposedOwnerRejects(environment) {
    environment.__tachyonNativeHostCall = () => JSON.stringify({ task: 4 })
    const pending = NativeTaskAnswersCheck.call(environment)
    await new Promise((resolve) => setTimeout(resolve, 10))
    environment.tasks.cancel(NativeTaskAnswersCheck.owner)
    await assert.rejects(pending, /owner disposed/)
    assert.equal(environment.tasks.waiting.size, 0)
  }

  static async otherOwnersAreIgnored(environment) {
    environment.__tachyonNativeHostCall = () => JSON.stringify({ task: 5 })
    const pending = NativeTaskAnswersCheck.call(environment)
    await new Promise((resolve) => setTimeout(resolve, 10))
    environment.tasks.done(NativeTaskAnswersCheck.signal(5, { value: 'forged' }, '00000000-0000-4000-8000-000000000072'))
    environment.tasks.done({ ...NativeTaskAnswersCheck.signal(5, { value: 'not text' }), answer: { value: 1 } })
    environment.tasks.done(NativeTaskAnswersCheck.signal(5, { value: 'owned' }))
    assert.equal(await pending, 'owned')
    environment.tasks.cancel('00000000-0000-4000-8000-000000000072')
    assert.equal(environment.tasks.early.size, 0)
  }

  static async synchronousAnswersStayTheSame(environment) {
    environment.__tachyonNativeHostCall = () => JSON.stringify({ value: 7 })
    assert.equal(await NativeTaskAnswersCheck.call(environment), 7)
    environment.__tachyonNativeHostCall = () => JSON.stringify({ error: 'Native companion operation failed.' })
    await assert.rejects(NativeTaskAnswersCheck.call(environment), /Tac native call failed/)
  }
}

await NativeTaskAnswersCheck.run()
