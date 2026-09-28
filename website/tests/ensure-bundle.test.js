import { describe, expect, test } from 'bun:test'
import { mkdtemp, rm, utimes, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { newestFile } from './helpers/ensure-bundle.js'

describe('the website tests build with the newest ty', () => {
  test('an old release ty never hides a newer debug ty', async () => {
    const folder = await mkdtemp(join(tmpdir(), 'tachyon-ty-choice-'))
    const release = join(folder, 'release-ty')
    const debug = join(folder, 'debug-ty')
    const missing = join(folder, 'missing-ty')
    await writeFile(release, '')
    await writeFile(debug, '')
    const longAgo = new Date(1_000_000)
    await utimes(release, longAgo, longAgo)
    expect(await newestFile([release, debug, missing])).toBe(debug)
    expect(await newestFile([missing])).toBeUndefined()
    await rm(folder, { recursive: true, force: true })
  })
})
