import { describe, expect, test } from 'bun:test'
import { existsSync, mkdtempSync, readdirSync, rmSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { TacRubyWarmUp } from '../scripts/tac-ruby-warm-up.mjs'

class RecordedCommands {
  /** @type {{command: string, args: string[], timeout: number}[]} */
  calls = []
  /** @type {string[]} */
  reports = []
  scratchRoot = mkdtempSync(path.join(tmpdir(), 'tac-ruby-warm-up-test-'))

  /**
   * @param {(args: string[]) => Promise<string>} answer
   * @returns {TacRubyWarmUp}
   */
  warmUp(answer) {
    const runCommand = (/** @type {string} */ command, /** @type {string[]} */ args, /** @type {{timeout: number}} */ options) => {
      this.calls.push({ command, args, timeout: options.timeout })
      return answer(args)
    }
    return new TacRubyWarmUp(runCommand, '/bin/ty', this.scratchRoot, text => this.reports.push(text))
  }

  remove() {
    rmSync(this.scratchRoot, { recursive: true, force: true })
  }
}

describe('verify-snippets builds the Tac Ruby runtime once before the first Ruby example', () => {
  test('a Ruby step needs the warm-up', () => {
    expect([TacRubyWarmUp.needs('iteration [rb]'), TacRubyWarmUp.needs('annotations / Roles: Tac [rb native]')]).toEqual([true, true])
  })

  test('a step in another language does not need the warm-up', () => {
    expect([TacRubyWarmUp.needs('iteration [go]'), TacRubyWarmUp.needs('iteration [rs]'), TacRubyWarmUp.needs('shared-data')]).toEqual([false, false, false])
  })

  test('the warm-up scaffolds a Tac Ruby project and bundles it under the long build limit', async () => {
    const recorded = new RecordedCommands()
    await recorded.warmUp(async () => '').ready()
    recorded.remove()
    expect(recorded.calls.map(call => [call.command, call.args[0], call.args.slice(2), call.timeout])).toEqual([
      ['/bin/ty', 'init', ['--tac', 'rb', '--yon', 'js', '--skip-tools'], TacRubyWarmUp.scaffoldLimitMilliseconds],
      ['/bin/ty', 'bundle', [], TacRubyWarmUp.buildLimitMilliseconds],
    ])
  })

  test('the build limit is longer than the two minutes each example gets', () => {
    expect(TacRubyWarmUp.buildLimitMilliseconds).toBeGreaterThan(120_000)
  })

  test('Ruby steps that start together share one warm-up', async () => {
    const recorded = new RecordedCommands()
    const warmUp = recorded.warmUp(async () => '')
    await Promise.all([warmUp.ready(), warmUp.ready(), warmUp.ready()])
    await warmUp.ready()
    recorded.remove()
    expect([recorded.calls.length, recorded.reports.length]).toEqual([2, 1])
  })

  test('a finished warm-up reports its time', async () => {
    const recorded = new RecordedCommands()
    await recorded.warmUp(async () => '').ready()
    recorded.remove()
    expect(recorded.reports[0]).toMatch(/^ {2}ok {2}Tac Ruby runtime, built once before the first Ruby example \[rb\] in \d+\.\ds$/)
  })

  test('a failed warm-up is reported and leaves each Ruby example to report its own result', async () => {
    const recorded = new RecordedCommands()
    const warmUp = recorded.warmUp(async args => {
      throw new Error(`${args[0]}: emcc was not found`)
    })
    await warmUp.ready()
    recorded.remove()
    expect([recorded.calls.length, recorded.reports[0]]).toEqual([1, expect.stringContaining('failed after')])
  })

  test('the failure report names the cause', async () => {
    const recorded = new RecordedCommands()
    await recorded.warmUp(async () => {
      throw new Error('emcc was not found')
    }).ready()
    recorded.remove()
    expect(recorded.reports[0]).toContain('each Ruby example reports its own result: Error: emcc was not found')
  })

  test('the scratch project is removed after the warm-up', async () => {
    const recorded = new RecordedCommands()
    const warmUp = recorded.warmUp(async () => '')
    await warmUp.ready()
    const left = readdirSync(recorded.scratchRoot)
    const projectWasInScratch = recorded.calls[0].args[1].startsWith(recorded.scratchRoot)
    recorded.remove()
    expect([left, projectWasInScratch, existsSync(recorded.calls[0].args[1])]).toEqual([[], true, false])
  })
})
