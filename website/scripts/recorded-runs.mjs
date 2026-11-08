import { createHash } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

/**
 * @typedef {{ method: string, path: string, headers: Record<string, string>, body?: string }} RecordedRequest
 * @typedef {{ status: number, contentType: string, body: string }} RecordedResponse
 * @typedef {{ request: RecordedRequest, response: RecordedResponse }} RecordedExchange
 * @typedef {{ source: string, code: string }} RecordedInput
 * @typedef {{ schema: number, handler: string, inputs: Record<string, string>, exchanges: RecordedExchange[] }} Recording
 * @typedef {{ schema: number, exchanges: RecordedExchange[] }} PublishedRun
 */

export class RecordedRuns {
  static SCHEMA = 1
  static ID_LENGTH = 16
  static SUFFIX = '.json'
  static site = new RecordedRuns(fileURLToPath(new URL('..', import.meta.url)))

  /** @param {string} website */
  constructor(website) {
    this.contentFolder = path.join(website, 'content')
    this.recordingFolder = path.join(website, 'runs')
    this.publishedFolder = path.join(website, 'client', 'shared', 'data', 'runs')
  }

  /**
   * @param {string} source
   * @returns {string}
   */
  static idFor(source) {
    return RecordedRuns.hash(source).slice(0, RecordedRuns.ID_LENGTH)
  }

  /**
   * @param {string} source
   * @param {RecordedInput[]} inputs
   * @param {RecordedExchange[]} exchanges
   * @returns {boolean}
   */
  write(source, inputs, exchanges) {
    const text = RecordedRuns.render(RecordedRuns.recording(source, inputs, exchanges))
    const file = this.recordingPath(source)
    mkdirSync(path.dirname(file), { recursive: true })
    return RecordedRuns.writeWhenChanged(file, text)
  }

  /**
   * @param {string} source
   * @returns {string | undefined}
   */
  currentId(source) {
    const recording = RecordedRuns.read(this.recordingPath(source))
    return this.idWhenCurrent(recording, source)
  }

  /** @returns {Map<string, string>} */
  published() {
    const recordings = RecordedRuns.recordingFiles(this.recordingFolder).map(file => RecordedRuns.read(file))
    const current = recordings.filter(recording => this.isCurrent(recording))
    return new Map(current.map(recording => [RecordedRuns.publishedName(recording), RecordedRuns.publishedText(recording)]))
  }

  /** @returns {string[]} */
  stale() {
    const recordings = RecordedRuns.recordingFiles(this.recordingFolder).map(file => RecordedRuns.read(file))
    return recordings.filter(recording => !this.isCurrent(recording)).map(recording => recording.handler)
  }

  /**
   * @param {string} source
   * @returns {string}
   */
  recordingPath(source) {
    return path.join(this.recordingFolder, `${source}${RecordedRuns.SUFFIX}`)
  }

  /**
   * @param {string} text
   * @returns {string}
   */
  static hash(text) {
    return createHash('sha256').update(text).digest('hex')
  }

  /**
   * @param {string} source
   * @param {RecordedInput[]} inputs
   * @param {RecordedExchange[]} exchanges
   * @returns {Recording}
   */
  static recording(source, inputs, exchanges) {
    const sorted = [...inputs].sort((first, second) => first.source.localeCompare(second.source))
    const hashes = Object.fromEntries(sorted.map(input => [input.source, RecordedRuns.hash(input.code)]))
    return { schema: RecordedRuns.SCHEMA, handler: source, inputs: hashes, exchanges }
  }

  /**
   * @param {object} value
   * @returns {string}
   */
  static render(value) {
    return `${JSON.stringify(value, null, 2)}\n`
  }

  /**
   * @param {string} file
   * @param {string} text
   * @returns {boolean}
   */
  static writeWhenChanged(file, text) {
    if (RecordedRuns.textOf(file) === text) return false
    writeFileSync(file, text)
    return true
  }

  /**
   * @param {string} file
   * @returns {string | undefined}
   */
  static textOf(file) {
    if (!existsSync(file)) return undefined
    return readFileSync(file, 'utf8')
  }

  /**
   * @param {string} file
   * @returns {Recording | undefined}
   */
  static read(file) {
    const text = RecordedRuns.textOf(file)
    if (text === undefined) return undefined
    return JSON.parse(text)
  }

  /**
   * @param {Recording | undefined} recording
   * @param {string} source
   * @returns {string | undefined}
   */
  idWhenCurrent(recording, source) {
    if (!this.isCurrent(recording)) return undefined
    return RecordedRuns.idFor(source)
  }

  /**
   * @param {Recording | undefined} recording
   * @returns {boolean}
   */
  isCurrent(recording) {
    if (recording?.schema !== RecordedRuns.SCHEMA) return false
    return Object.entries(recording.inputs).every(([source, hash]) => this.inputMatches(source, hash))
  }

  /**
   * @param {string} source
   * @param {string} hash
   * @returns {boolean}
   */
  inputMatches(source, hash) {
    const text = RecordedRuns.textOf(path.join(this.contentFolder, source))
    return text !== undefined && RecordedRuns.hash(text) === hash
  }

  /**
   * @param {string} folder
   * @returns {string[]}
   */
  static recordingFiles(folder) {
    if (!existsSync(folder)) return []
    return readdirSync(folder, { recursive: true, encoding: 'utf8' })
      .filter(name => name.endsWith(RecordedRuns.SUFFIX))
      .map(name => path.join(folder, name))
      .sort()
  }

  /**
   * @param {Recording} recording
   * @returns {string}
   */
  static publishedName(recording) {
    return `${RecordedRuns.idFor(recording.handler)}${RecordedRuns.SUFFIX}`
  }

  /**
   * @param {Recording} recording
   * @returns {string}
   */
  static publishedText(recording) {
    /** @type {PublishedRun} */
    const run = { schema: recording.schema, exchanges: recording.exchanges }
    return RecordedRuns.render(run)
  }
}
