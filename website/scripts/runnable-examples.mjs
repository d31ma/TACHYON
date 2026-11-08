import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { RecordedRuns } from './recorded-runs.mjs'

/**
 * @typedef {{ method: string, path: string }} RunnableRoute
 * @typedef {{ name: string, code: string }} RunnableFile
 * @typedef {{ schema: number, runId: string, source: string, language: string, routes: RunnableRoute[] }} RunnableExample
 */

export class RunnableExamples {
  static SCHEMA = 1
  static DESCRIPTION = 'runnable.json'
  static PROJECT = 'project'

  /** @param {string} folder */
  constructor(folder) {
    this.folder = folder
  }

  /**
   * @param {string} source
   * @param {RunnableFile[]} files
   * @param {RunnableRoute[]} routes
   * @returns {string}
   */
  stage(source, files, routes) {
    const runId = RecordedRuns.idFor(source)
    const exampleFolder = path.join(this.folder, runId)
    rmSync(exampleFolder, { recursive: true, force: true })
    for (const file of files) RunnableExamples.writeFile(path.join(exampleFolder, RunnableExamples.PROJECT), file)
    writeFileSync(path.join(exampleFolder, RunnableExamples.DESCRIPTION), RunnableExamples.render(RunnableExamples.description(runId, source, routes)))
    return runId
  }

  /** @returns {RunnableExample[]} */
  list() {
    if (!existsSync(this.folder)) return []
    const runIds = readdirSync(this.folder).filter(name => existsSync(path.join(this.folder, name, RunnableExamples.DESCRIPTION)))
    return runIds.sort().map(runId => JSON.parse(readFileSync(path.join(this.folder, runId, RunnableExamples.DESCRIPTION), 'utf8')))
  }

  /**
   * @param {string} projectFolder
   * @param {RunnableFile} file
   */
  static writeFile(projectFolder, file) {
    const target = path.join(projectFolder, file.name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, file.code)
  }

  /**
   * @param {string} runId
   * @param {string} source
   * @param {RunnableRoute[]} routes
   * @returns {RunnableExample}
   */
  static description(runId, source, routes) {
    return { schema: RunnableExamples.SCHEMA, runId, source, language: path.extname(source).slice(1), routes }
  }

  /**
   * @param {object} value
   * @returns {string}
   */
  static render(value) {
    return `${JSON.stringify(value, null, 2)}\n`
  }
}
