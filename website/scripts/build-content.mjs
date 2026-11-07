import { readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { SiteContent } from './site-content.mjs'

class BuildContent {
  static checkOnly = process.argv.includes('--check')

  static run() {
    const staleFiles = []
    for (const file of SiteContent.generatedFiles()) {
      const expected = SiteContent.render(file)
      const outputPath = SiteContent.outputPath(file)
      if (BuildContent.currentText(outputPath) === expected) continue
      staleFiles.push(path.relative(SiteContent.website, outputPath))
      if (!BuildContent.checkOnly) writeFileSync(outputPath, expected)
    }
    BuildContent.report(staleFiles)
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
