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
    const runs = new RecordedRuns(website)
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
  test('a recording of unchanged files gives the handler a run id', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    expect(runs.currentId(RunFixture.handler)).toBe(RecordedRuns.idFor(RunFixture.handler))
  })

  test('the run id is the first 16 hex digits of the handler path hash', () => {
    expect(RecordedRuns.idFor(RunFixture.handler)).toMatch(/^[0-9a-f]{16}$/)
  })

  test('a handler without a recording has no run id', () => {
    const runs = RunFixture.runs()
    expect(runs.currentId(RunFixture.handler)).toBeUndefined()
  })

  test('changing an input file makes the recording stale', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    RunFixture.writeContent(runs, RunFixture.model, 'class Message:\n    name: str\n')
    expect(runs.currentId(RunFixture.handler)).toBeUndefined()
    expect(runs.stale()).toEqual([RunFixture.handler])
  })

  test('removing an input file makes the recording stale', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    rmSync(path.join(runs.contentFolder, RunFixture.model))
    expect(runs.currentId(RunFixture.handler)).toBeUndefined()
  })

  test('a recording from another schema is stale', () => {
    const runs = RunFixture.runs()
    RunFixture.record(runs)
    const file = runs.recordingPath(RunFixture.handler)
    writeFileSync(file, readFileSync(file, 'utf8').replace('"schema": 1', '"schema": 2'))
    expect(runs.currentId(RunFixture.handler)).toBeUndefined()
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
