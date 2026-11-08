import { afterEach, describe, expect, test } from 'bun:test'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { RecordedRuns } from '../scripts/recorded-runs.mjs'

class RunFixture {
  /** @type {string[]} */
  static folders = []
  static handler = 'features/echo/server/controllers/EchoController.py'
  static model = 'features/echo/server/models/Message.py'
  static handlerCode = 'class EchoController: pass\n'
  static modelCode = 'class Message: pass\n'
  /** @type {import('../scripts/recorded-runs.mjs').RecordedExchange[]} */
  static exchanges = [{
    request: { method: 'POST', path: '/echo', headers: {}, body: '{"name":"snippet"}' },
    response: { status: 200, contentType: 'application/json', body: '{"name":"snippet"}' },
  }]

  /** @returns {RecordedRuns} */
  static runs() {
    const website = mkdtempSync(path.join(tmpdir(), 'recorded-runs-'))
    RunFixture.folders.push(website)
    const runs = new RecordedRuns(website, path.join(website, 'runs'))
    RunFixture.writeContent(runs, RunFixture.handler, RunFixture.handlerCode)
    RunFixture.writeContent(runs, RunFixture.model, RunFixture.modelCode)
    return runs
  }

  /**
   * @param {RecordedRuns} runs
   * @param {string} source
   * @param {string} code
   */
  static writeContent(runs, source, code) {
    const file = path.join(runs.contentFolder, source)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, code)
  }

  /** @param {RecordedRuns} runs */
  static record(runs) {
    const inputs = [
      { source: RunFixture.model, code: RunFixture.modelCode },
      { source: RunFixture.handler, code: RunFixture.handlerCode },
    ]
    return runs.write(RunFixture.handler, inputs, RunFixture.exchanges)
  }
}

afterEach(() => {
  for (const folder of RunFixture.folders.splice(0)) rmSync(folder, { recursive: true, force: true })
})

describe('RecordedRuns', () => {
  test('a recording of unchanged files is published', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    expect([...runs.published().keys()]).toEqual([`${RecordedRuns.idFor(RunFixture.handler)}.json`])
  })

  test('the run id is the first 16 hex digits of the handler path hash', () => {
    expect(RecordedRuns.idFor(RunFixture.handler)).toMatch(/^[0-9a-f]{16}$/)
  })

  test('a missing recording folder publishes nothing', () => {
    const runs = RunFixture.runs()
    expect(runs.published().size).toBe(0)
    expect(runs.stale()).toEqual([])
  })

  test('a controller with an HTTP route can be recorded', () => {
    expect(RecordedRuns.isRecordable('server/controllers/EchoController.py', '@request_route("/echo")\nclass EchoController: pass')).toBe(true)
  })

  test('a socket controller cannot be recorded', () => {
    expect(RecordedRuns.isRecordable('server/controllers/ChatController.ts', '@SocketRoute("/chat")\nexport default class ChatController {}')).toBe(false)
  })

  test('a file outside server/controllers cannot be recorded', () => {
    expect(RecordedRuns.isRecordable('server/models/Message.py', 'class Message: pass')).toBe(false)
  })

  test('the scratch folder for recordings is hidden from staging cleanup', () => {
    expect(path.basename(RecordedRuns.scratchFolder('/tmp/tachyon-snippets'))).toBe('.runs')
  })

  test('changing an input file makes the recording stale', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    RunFixture.writeContent(runs, RunFixture.model, 'class Message:\n    name: str\n')
    expect(runs.published().size).toBe(0)
    expect(runs.stale()).toEqual([RunFixture.handler])
  })

  test('removing an input file makes the recording stale', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    rmSync(path.join(runs.contentFolder, RunFixture.model))
    expect(runs.published().size).toBe(0)
  })

  test('a recording from another schema is stale', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    const file = runs.recordingPath(RunFixture.handler)
    writeFileSync(file, readFileSync(file, 'utf8').replace('"schema": 1', '"schema": 2'))
    expect(runs.published().size).toBe(0)
  })

  test('writing the same recording again changes nothing', () => {
    const runs = RunFixture.runs()
    expect(RunFixture.record(runs)).toBe(true)
    expect(RunFixture.record(runs)).toBe(false)
  })

  test('the recording keeps the input hashes in path order', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    const recording = JSON.parse(readFileSync(runs.recordingPath(RunFixture.handler), 'utf8'))
    expect(Object.keys(recording.inputs)).toEqual([RunFixture.handler, RunFixture.model])
  })

  test('published runs hold only the exchanges, named by run id', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    const published = runs.published()
    const name = `${RecordedRuns.idFor(RunFixture.handler)}.json`
    expect([...published.keys()]).toEqual([name])
    expect(JSON.parse(String(published.get(name)))).toEqual({ schema: 1, exchanges: RunFixture.exchanges })
  })

  test('stale recordings are not published', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    RunFixture.writeContent(runs, RunFixture.handler, 'class EchoController:\n    changed = True\n')
    expect(runs.published().size).toBe(0)
  })

  test('the recording sits under runs at the handler path', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    expect(existsSync(path.join(runs.recordingFolder, `${RunFixture.handler}.json`))).toBe(true)
  })
})
