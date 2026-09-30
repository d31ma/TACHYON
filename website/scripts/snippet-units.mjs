import { readFileSync } from 'node:fs'
import path from 'node:path'
import { SiteContent } from './site-content.mjs'

/** @typedef {{ file: string, text: string }} CodeFile */
/** @typedef {{ name: string, extension: string, code: string, named: boolean }} UnitFile */
/** @typedef {{ page: string, id: string, title: string, text: string, files: UnitFile[], javascriptOnly?: string }} SnippetUnit */
/** @typedef {{ name: string, code: CodeFile | string }} ContentFile */
/** @typedef {{ heading: string, body?: string, code?: CodeFile | string, files?: ContentFile[] }} ContentSection */
/** @typedef {{ id: string, title: string, summary?: string, files?: ContentFile[], javascriptOnly?: string }} ContentFeature */

export class SnippetUnits {
  static docsPage = 'docs/page.json'
  static featuresPage = 'features/page.json'
  static docs = 'docs'
  static features = 'features'
  static proseFile = /^[^.]+$|\.(txt|md)$/

  /** @returns {SnippetUnit[]} */
  static collect() {
    const docs = SnippetUnits.load(SnippetUnits.docsPage)
    const features = SnippetUnits.load(SnippetUnits.featuresPage)
    const units = [...SnippetUnits.docsUnits(docs.topics), ...SnippetUnits.featureUnits(features.features)]
    return units.filter(unit => unit.files.length > 0)
  }

  /**
   * @param {string} page
   * @returns {any}
   */
  static load(page) {
    return SiteContent.load(path.join(SiteContent.contentFolder, page), SnippetUnits.readCode)
  }

  /**
   * @param {string} file
   * @returns {CodeFile}
   */
  static readCode(file) {
    return { file, text: readFileSync(file, 'utf8') }
  }

  /**
   * @param {Record<string, { sections?: ContentSection[] }>} topics
   * @returns {SnippetUnit[]}
   */
  static docsUnits(topics) {
    return Object.entries(topics).flatMap(([slug, topic]) => (topic.sections ?? []).map(section => SnippetUnits.sectionUnit(slug, section)))
  }

  /**
   * @param {string} slug
   * @param {ContentSection} section
   * @returns {SnippetUnit}
   */
  static sectionUnit(slug, section) {
    return {
      page: SnippetUnits.docs,
      id: slug,
      title: section.heading,
      text: SnippetUnits.textOf(section.body ?? ''),
      files: [...SnippetUnits.namedFiles(section.files ?? []), ...SnippetUnits.unnamedFiles(section.code)],
    }
  }

  /**
   * @param {ContentFeature[]} features
   * @returns {SnippetUnit[]}
   */
  static featureUnits(features) {
    return features.map(feature => SnippetUnits.featureUnit(feature))
  }

  /**
   * @param {ContentFeature} feature
   * @returns {SnippetUnit}
   */
  static featureUnit(feature) {
    const files = feature.files ?? []
    return {
      page: SnippetUnits.features,
      id: feature.id,
      title: feature.title,
      text: [feature.summary ?? '', ...SnippetUnits.proseTexts(files)].join('\n'),
      files: SnippetUnits.namedFiles(files),
      javascriptOnly: feature.javascriptOnly,
    }
  }

  /**
   * @param {ContentFile[]} files
   * @returns {string[]}
   */
  static proseTexts(files) {
    return files.filter(file => SnippetUnits.proseFile.test(file.name)).map(file => SnippetUnits.textOf(file.code))
  }

  /**
   * @param {ContentFile[]} files
   * @returns {UnitFile[]}
   */
  static namedFiles(files) {
    return files.map(file => SnippetUnits.unitFile(file.name, file.code, true))
  }

  /**
   * @param {CodeFile | string | undefined} code
   * @returns {UnitFile[]}
   */
  static unnamedFiles(code) {
    if (typeof code !== 'object') return []
    return [SnippetUnits.unitFile(path.basename(code.file), code, false)]
  }

  /**
   * @param {string} name
   * @param {CodeFile | string} code
   * @param {boolean} named
   * @returns {UnitFile}
   */
  static unitFile(name, code, named) {
    return { name, extension: SnippetUnits.extensionOf(name), code: SnippetUnits.textOf(code), named }
  }

  /**
   * @param {string} name
   * @returns {string}
   */
  static extensionOf(name) {
    return path.extname(name).slice(1).toLowerCase()
  }

  /**
   * @param {CodeFile | string} value
   * @returns {string}
   */
  static textOf(value) {
    if (typeof value === 'string') return value
    return value.text
  }
}
