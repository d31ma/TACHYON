import { existsSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { RecordedRuns } from './recorded-runs.mjs'
import { SiteContent } from './site-content.mjs'

class BuildContent {
  static checkOnly = process.argv.includes('--check')

  static run() {
    const staleFiles = [...BuildContent.syncGeneratedFiles(), ...BuildContent.syncPublishedRuns()]
    BuildContent.reportOutdatedRecordings(RecordedRuns.site.stale())
    BuildContent.report(staleFiles)
  }

  /** @returns {string[]} */
  static syncGeneratedFiles() {
    const expected = SiteContent.generatedFiles().map(file => [SiteContent.outputPath(file), SiteContent.render(file)])
    return BuildContent.syncFiles(new Map(/** @type {[string, string][]} */ (expected)))
  }

  /** @returns {string[]} */
  static syncPublishedRuns() {
    const published = RecordedRuns.site.published()
    const expected = [...published].map(([name, text]) => [path.join(RecordedRuns.site.publishedFolder, name), text])
    const changed = BuildContent.syncFiles(new Map(/** @type {[string, string][]} */ (expected)))
    return [...changed, ...BuildContent.removeUnpublished(new Set(published.keys()))]
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
   * @param {Set<string>} keptNames
   * @returns {string[]}
   */
  static removeUnpublished(keptNames) {
    const extraNames = BuildContent.publishedNames().filter(name => !keptNames.has(name))
    for (const name of extraNames) BuildContent.removeUnlessChecking(path.join(RecordedRuns.site.publishedFolder, name))
    return extraNames.map(name => path.relative(SiteContent.website, path.join(RecordedRuns.site.publishedFolder, name)))
  }

  /** @returns {string[]} */
  static publishedNames() {
    if (!existsSync(RecordedRuns.site.publishedFolder)) return []
    return readdirSync(RecordedRuns.site.publishedFolder).filter(name => name.endsWith(RecordedRuns.SUFFIX))
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

  /** @param {string} outputPath */
  static removeUnlessChecking(outputPath) {
    if (BuildContent.checkOnly) return
    rmSync(outputPath)
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
    console.warn(`These recorded runs no longer match their snippet files and are not published: ${handlers.join(', ')}`)
    console.warn('Record them again into website/runs with: TAC_SNIPPETS_RECORD=1 bun scripts/verify-snippets.mjs')
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
