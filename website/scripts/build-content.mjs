import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { RecordedRuns } from './recorded-runs.mjs'
import { SiteContent } from './site-content.mjs'

class BuildContent {
  static RUNS_OPTION = '--runs'
  static checkOnly = process.argv.includes('--check')

  static run() {
    const staleFiles = BuildContent.syncFiles(BuildContent.generatedFiles())
    BuildContent.report(staleFiles)
    BuildContent.publishUnlessChecking(RecordedRuns.forSite(BuildContent.runsFolder(process.argv)))
  }

  /** @returns {Map<string, string>} */
  static generatedFiles() {
    const expected = SiteContent.generatedFiles().map(file => [SiteContent.outputPath(file), SiteContent.render(file)])
    return new Map(/** @type {[string, string][]} */ (expected))
  }

  /**
   * @param {string[]} argumentsList
   * @returns {string}
   */
  static runsFolder(argumentsList) {
    const index = argumentsList.indexOf(BuildContent.RUNS_OPTION)
    if (index === -1) return RecordedRuns.sampleFolder
    return path.resolve(argumentsList[index + 1] ?? '')
  }

  /** @param {RecordedRuns} runs */
  static publishUnlessChecking(runs) {
    if (BuildContent.checkOnly) return
    BuildContent.publishRuns(runs)
  }

  /** @param {RecordedRuns} runs */
  static publishRuns(runs) {
    const published = runs.published()
    const expected = [...published].map(([name, text]) => [path.join(runs.publishedFolder, name), text])
    BuildContent.syncFiles(new Map(/** @type {[string, string][]} */ (expected)))
    BuildContent.removeUnpublished(runs.publishedFolder, new Set(published.keys()))
    console.log(`Published ${published.size} recorded runs from ${runs.recordingFolder} to ${path.relative(SiteContent.website, runs.publishedFolder)}.`)
    BuildContent.reportOutdatedRecordings(runs.stale())
  }

  /**
   * @param {Map<string, string>} expected
   * @returns {string[]}
   */
  static syncFiles(expected) {
    const staleFiles = []
    for (const [outputPath, text] of expected) {
      if (BuildContent.currentText(outputPath) === text) continue
      staleFiles.push(path.relative(SiteContent.website, outputPath))
      BuildContent.writeUnlessChecking(outputPath, text)
    }
    return staleFiles
  }

  /**
   * @param {string} folder
   * @param {Set<string>} keptNames
   */
  static removeUnpublished(folder, keptNames) {
    const extraNames = BuildContent.publishedNames(folder).filter(name => !keptNames.has(name))
    for (const name of extraNames) rmSync(path.join(folder, name))
  }

  /**
   * @param {string} folder
   * @returns {string[]}
   */
  static publishedNames(folder) {
    if (!existsSync(folder)) return []
    return readdirSync(folder).filter(name => name.endsWith(RecordedRuns.SUFFIX))
  }

  /**
   * @param {string} outputPath
   * @param {string} text
   */
  static writeUnlessChecking(outputPath, text) {
    if (BuildContent.checkOnly) return
    mkdirSync(path.dirname(outputPath), { recursive: true })
    writeFileSync(outputPath, text)
  }

  /**
   * @param {string} outputPath
   * @returns {string | undefined}
   */
  static currentText(outputPath) {
    try {
      return readFileSync(outputPath, 'utf8')
    } catch {
      return undefined
    }
  }

  /** @param {string[]} handlers */
  static reportOutdatedRecordings(handlers) {
    if (handlers.length === 0) return
    console.warn(`These recorded runs no longer match their example files and are not published: ${handlers.join(', ')}`)
  }

  /** @param {string[]} staleFiles */
  static report(staleFiles) {
    if (staleFiles.length === 0) {
      console.log('Generated site data matches website/content.')
      return
    }
    if (!BuildContent.checkOnly) {
      console.log(`Wrote ${staleFiles.join(', ')} from website/content.`)
      return
    }
    console.error(`These files do not match website/content: ${staleFiles.join(', ')}`)
    console.error('Edit the files under website/content, then run: bun run content')
    process.exitCode = 1
  }
}

BuildContent.run()
