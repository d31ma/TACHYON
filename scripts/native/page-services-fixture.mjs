import { cpSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

export class PageServicesFixture {
  static features = fileURLToPath(new URL('../../website/content/features', import.meta.url))
  static tallySteps = [
    ['wait', ['0 in the basket', 'Add two']],
    ['click', 'Add two'],
    ['wait', ['2 in the basket', '2 items']],
  ]
  static reportSteps = [
    ['wait', ['Report', 'Rename']],
    ['click', 'Rename'],
    ['wait', ['Quarterly report']],
    ['click', 'Resize'],
    ['size', [1280, 800]],
  ]

  /** @param {string} name @param {string} fallback */
  static list(name, fallback) {
    return (process.env[name] ?? fallback).split(',').map(item => item.trim()).filter(Boolean)
  }

  /** @param {string} project @param {string} feature @param {string} name @param {string} id */
  static stage(project, feature, name, id) {
    const source = path.join(this.features, feature, 'client')
    for (const entry of ['pages', 'components']) cpSync(path.join(source, entry), path.join(project, 'client', entry), { recursive: true })
    const manifest = JSON.parse(readFileSync(path.join(source, 'manifest.json'), 'utf8'))
    manifest.tac = { ...manifest.tac, application: { ...manifest.tac?.application, id } }
    manifest.name ??= name
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest, null, 2))
    return manifest.name.replace(/[^A-Za-z0-9]/g, '')
  }

  /** @param {string} project @param {string} feature @param {string} file */
  static copyService(project, feature, file) {
    const target = path.join(project, 'client/services', path.basename(file))
    mkdirSync(path.dirname(target), { recursive: true })
    cpSync(path.join(this.features, feature, file), target)
  }

  /** @param {string} project @param {string} language */
  static stageTally(project, language) {
    const application = this.stage(project, 'web-assembly-services', 'Tally', `dev.tachyon.tally-${language}`)
    this.copyService(project, 'web-assembly-services', `client/services/TallyService.${language}`)
    return application
  }

  /** @param {string} project @param {string} language */
  static stageWindowCaller(project, language) {
    const application = this.stage(project, 'window-controls', 'Catalog', `dev.tachyon.window-${language}`)
    this.copyService(project, 'window-controls', `client/services/ReportService.${language}`)
    return application
  }
}
