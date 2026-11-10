import { afterEach, describe, expect, test } from 'bun:test'
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { RecordedRuns } from '../scripts/recorded-runs.mjs'
import { RunnableExamples } from '../scripts/runnable-examples.mjs'

class StagingFixture {
  /** @type {string[]} */
  static folders = []
  static source = 'features/dynamic-segments/server/controllers/ItemsController.py'
  static files = [
    { name: 'server/controllers/ItemsController.py', code: 'class ItemsController: pass\n' },
    { name: 'server/models/Item.py', code: 'class Item: pass\n' },
  ]
  static routes = [{ method: 'GET', path: '/items/{id}' }]

  /** @returns {RunnableExamples} */
  static examples() {
    const folder = mkdtempSync(path.join(tmpdir(), 'runnable-examples-'))
    StagingFixture.folders.push(folder)
    return new RunnableExamples(folder)
  }
}

afterEach(() => {
  for (const folder of StagingFixture.folders.splice(0)) rmSync(folder, { recursive: true, force: true })
})

describe('RunnableExamples', () => {
  test('staging writes the project under the run id', () => {
    const examples = StagingFixture.examples()
    const runId = examples.stage(StagingFixture.source, StagingFixture.files, StagingFixture.routes)
    expect(runId).toBe(RecordedRuns.idFor(StagingFixture.source))
    expect(readFileSync(path.join(examples.folder, runId, 'project', 'server/models/Item.py'), 'utf8')).toBe('class Item: pass\n')
  })

  test('the description names the source, language and route templates', () => {
    const examples = StagingFixture.examples()
    examples.stage(StagingFixture.source, StagingFixture.files, StagingFixture.routes)
    expect(examples.list()).toEqual([{ schema: 1, runId: RecordedRuns.idFor(StagingFixture.source), source: StagingFixture.source, language: 'py', routes: StagingFixture.routes }])
  })

  test('staging again replaces the old project', () => {
    const examples = StagingFixture.examples()
    const runId = examples.stage(StagingFixture.source, StagingFixture.files, StagingFixture.routes)
    examples.stage(StagingFixture.source, [StagingFixture.files[0]], StagingFixture.routes)
    expect(existsSync(path.join(examples.folder, runId, 'project', 'server/models/Item.py'))).toBe(false)
  })

  test('a missing folder lists nothing', () => {
    expect(new RunnableExamples(path.join(tmpdir(), 'no-such-runnable-folder')).list()).toEqual([])
  })
})
