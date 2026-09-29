import { readFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

/** @typedef {{ source: string, output: string, escapeNonAscii: boolean, omit?: string[] }} GeneratedFile */

export class SiteContent {
  static website = fileURLToPath(new URL('..', import.meta.url))
  static contentFolder = path.join(SiteContent.website, 'content')
  static listFile = path.join(SiteContent.contentFolder, 'generated-files.json')

  /** @returns {GeneratedFile[]} */
  static generatedFiles() {
    return JSON.parse(readFileSync(SiteContent.listFile, 'utf8'))
  }

  /**
   * @param {string} name the output file name, such as docs.json
   * @returns {any}
   */
  static data(name) {
    const file = SiteContent.generatedFiles().find(entry => path.basename(entry.output) === name)
    if (!file) throw new Error(`website/content/generated-files.json does not list ${name}`)
    return SiteContent.value(file)
  }

  /**
   * @param {GeneratedFile} file
   * @returns {any}
   */
  static value(file) {
    const value = SiteContent.load(path.join(SiteContent.contentFolder, file.source))
    return SiteContent.without(value, file.omit ?? [])
  }

  /**
   * @param {any} value
   * @param {string[]} keys the object keys to leave out, at any depth
   * @returns {any}
   */
  static without(value, keys) {
    if (Array.isArray(value)) return value.map(item => SiteContent.without(item, keys))
    if (value === null || typeof value !== 'object') return value
    const kept = Object.entries(value).filter(([key]) => !keys.includes(key))
    return Object.fromEntries(kept.map(([key, item]) => [key, SiteContent.without(item, keys)]))
  }

  /**
   * @param {GeneratedFile} file
   * @returns {string}
   */
  static render(file) {
    const text = `${JSON.stringify(SiteContent.value(file), null, 2)}\n`
    if (!file.escapeNonAscii) return text
    return SiteContent.escapeNonAscii(text)
  }

  /**
   * @param {GeneratedFile} file
   * @returns {string}
   */
  static outputPath(file) {
    return path.join(SiteContent.website, file.output)
  }

  /**
   * @param {string} jsonFile
   * @param {(file: string) => any} [readFile]
   * @returns {any}
   */
  static load(jsonFile, readFile = SiteContent.readText) {
    const value = JSON.parse(readFileSync(jsonFile, 'utf8'))
    return SiteContent.resolve(value, path.dirname(jsonFile), readFile)
  }

  /**
   * @param {any} value
   * @param {string} folder
   * @param {(file: string) => any} [readFile]
   * @returns {any}
   */
  static resolve(value, folder, readFile = SiteContent.readText) {
    if (Array.isArray(value)) return value.map(item => SiteContent.resolve(item, folder, readFile))
    if (value === null || typeof value !== 'object') return value
    if (SiteContent.isReference(value, 'include')) return SiteContent.load(SiteContent.inside(folder, value.include), readFile)
    if (SiteContent.isReference(value, 'file')) return readFile(SiteContent.inside(folder, value.file))
    /** @type {Record<string, any>} */
    const result = {}
    for (const [key, item] of Object.entries(value)) result[key] = SiteContent.resolve(item, folder, readFile)
    return result
  }

  /**
   * @param {string} file
   * @returns {string}
   */
  static readText(file) {
    return readFileSync(file, 'utf8')
  }

  /**
   * @param {string} jsonFile
   * @returns {string[]}
   */
  static referencedFiles(jsonFile) {
    const files = [jsonFile]
    SiteContent.collectReferences(JSON.parse(readFileSync(jsonFile, 'utf8')), path.dirname(jsonFile), files)
    return files
  }

  /**
   * @param {any} value
   * @param {string} folder
   * @param {string[]} files
   */
  static collectReferences(value, folder, files) {
    if (value === null || typeof value !== 'object') return
    if (SiteContent.isReference(value, 'include')) {
      files.push(...SiteContent.referencedFiles(SiteContent.inside(folder, value.include)))
      return
    }
    if (SiteContent.isReference(value, 'file')) {
      files.push(SiteContent.inside(folder, value.file))
      return
    }
    for (const item of Object.values(value)) SiteContent.collectReferences(item, folder, files)
  }

  /**
   * @param {Record<string, any>} value
   * @param {string} key
   * @returns {boolean}
   */
  static isReference(value, key) {
    const keys = Object.keys(value)
    return keys.length === 1 && keys[0] === key && typeof value[key] === 'string'
  }

  /**
   * @param {string} folder
   * @param {string} relativePath
   * @returns {string}
   */
  static inside(folder, relativePath) {
    const file = path.resolve(folder, relativePath)
    const fromContent = path.relative(SiteContent.contentFolder, file)
    if (path.isAbsolute(relativePath) || fromContent.startsWith('..') || path.isAbsolute(fromContent)) {
      throw new Error(`${relativePath} in ${folder} points outside website/content`)
    }
    return file
  }

  /**
   * @param {string} text
   * @returns {string}
   */
  static escapeNonAscii(text) {
    return text.replace(/[\u007f-\uffff]/g, character => `\\u${character.charCodeAt(0).toString(16).padStart(4, '0')}`)
  }
}
