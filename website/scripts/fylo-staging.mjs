import { existsSync, mkdirSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'

export class FyloStaging {
  static settingsFile = 'server/application.json'

  /**
   * @param {{name: string, code: string}[]} files
   * @returns {string[]}
   */
  static tools(files) {
    const settings = files.find(file => file.name === FyloStaging.settingsFile)
    if (!settings || !FyloStaging.isFylo(JSON.parse(settings.code))) return []
    return ['fylo']
  }

  /** @param {string} root */
  static prepare(root) {
    const settingsPath = path.join(root, FyloStaging.settingsFile)
    if (!existsSync(settingsPath)) return
    const settings = JSON.parse(readFileSync(settingsPath, 'utf8'))
    if (!FyloStaging.isFylo(settings)) return
    const dataRoot = `${root}.fylo`
    rmSync(dataRoot, { recursive: true, force: true })
    mkdirSync(dataRoot, { recursive: true })
    settings.datasource.url = `fylo://${realpathSync(dataRoot)}`
    writeFileSync(settingsPath, JSON.stringify(settings))
  }

  /**
   * @param {{datasource?: {driver?: string}}} settings
   * @returns {boolean}
   */
  static isFylo(settings) {
    return settings.datasource?.driver === 'fylo'
  }
}
