import { mkdtempSync, rmSync } from 'node:fs'
import path from 'node:path'

/** @typedef {(command: string, args: string[], options: {timeout: number}) => Promise<string>} CommandRunner */

export class TacRubyWarmUp {
  static label = 'Tac Ruby runtime, built once before the first Ruby example [rb]'
  static rubyStep = /\[rb[\] ]/
  static folderPrefix = 'tac-ruby-warm-up-'
  static scaffoldLimitMilliseconds = 60_000
  static buildLimitMilliseconds = 20 * 60_000
  static millisecondsPerSecond = 1000

  /** @type {CommandRunner} */
  #runCommand
  /** @type {string} */
  #binary
  /** @type {string} */
  #scratchRoot
  /** @type {(text: string) => void} */
  #report
  /** @type {Promise<void>|undefined} */
  #started

  /**
   * @param {CommandRunner} runCommand
   * @param {string} binary
   * @param {string} scratchRoot
   * @param {(text: string) => void} report
   */
  constructor(runCommand, binary, scratchRoot, report) {
    this.#runCommand = runCommand
    this.#binary = binary
    this.#scratchRoot = scratchRoot
    this.#report = report
  }

  /**
   * @param {string} label
   * @returns {boolean}
   */
  static needs(label) {
    return TacRubyWarmUp.rubyStep.test(label)
  }

  /** @returns {Promise<void>} */
  ready() {
    return this.#startOnce()
  }

  /** @returns {Promise<void>} */
  #startOnce() {
    this.#started ??= this.#warm(Date.now())
    return this.#started
  }

  /**
   * @param {number} startedAt
   * @returns {Promise<void>}
   */
  #warm(startedAt) {
    return this.#buildInScratchProject().then(
      () => this.#report(`  ok  ${TacRubyWarmUp.label} in ${TacRubyWarmUp.#secondsSince(startedAt)}`),
      error => this.#report(`  --  ${TacRubyWarmUp.label} failed after ${TacRubyWarmUp.#secondsSince(startedAt)}; each Ruby example reports its own result: ${error}`),
    )
  }

  /** @returns {Promise<void>} */
  async #buildInScratchProject() {
    const folder = mkdtempSync(path.join(this.#scratchRoot, TacRubyWarmUp.folderPrefix))
    try {
      await this.#scaffoldAndBundle(path.join(folder, 'app'))
    } finally {
      rmSync(folder, { recursive: true, force: true })
    }
  }

  /**
   * @param {string} project
   * @returns {Promise<void>}
   */
  async #scaffoldAndBundle(project) {
    const scaffold = ['init', project, '--tac', 'rb', '--yon', 'js', '--skip-tools']
    await this.#runCommand(this.#binary, scaffold, { timeout: TacRubyWarmUp.scaffoldLimitMilliseconds })
    await this.#runCommand(this.#binary, ['bundle', project], { timeout: TacRubyWarmUp.buildLimitMilliseconds })
  }

  /**
   * @param {number} startedAt
   * @returns {string}
   */
  static #secondsSince(startedAt) {
    return `${((Date.now() - startedAt) / TacRubyWarmUp.millisecondsPerSecond).toFixed(1)}s`
  }
}
