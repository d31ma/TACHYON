import { readFile, writeFile, readdir } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export class StyleBuild {
  static _start = '/* @tachyon-generated-tailwind:start */'
  static _end = '/* @tachyon-generated-tailwind:end */'
  static _properties = /@property\s+--[\w-]+\s*\{[^}]*\}/g

  /** @type {string} */
  _website
  /** @type {boolean} */
  _check
  /** @type {typeof import('@tailwindcss/node').compile} */
  _compile
  /** @type {typeof import('@tailwindcss/oxide').Scanner} */
  _Scanner
  /** @type {NodeJS.Require} */
  _require
  /** @type {Map<string, string>} */
  _registrations = new Map()

  /** @param {boolean} check */
  constructor(check) {
    this._website = fileURLToPath(new URL('../', import.meta.url))
    this._check = check
    this._require = StyleBuild._dependencyRequire()
    this._compile = this._require('@tailwindcss/node').compile
    this._Scanner = this._require('@tailwindcss/oxide').Scanner
  }

  /** @returns {Promise<void>} */
  static async run() {
    const build = new StyleBuild(process.argv.includes('--check'))
    const started = performance.now()
    const count = await build._components()
    await build._foundation()
    process.stdout.write(`${JSON.stringify({ components: count, registrations: build._registrations.size, check: build._check, seconds: (performance.now() - started) / 1000 })}\n`)
  }

  /** @returns {NodeJS.Require} */
  static _dependencyRequire() {
    return createRequire(StyleBuild._dependencyLocation())
  }

  /** @returns {string} */
  static _dependencyLocation() {
    if (process.env.TAC_WEBSITE_DEPENDENCIES) return resolve(process.env.TAC_WEBSITE_DEPENDENCIES, '../package.json')
    return fileURLToPath(import.meta.url)
  }

  /** @returns {Promise<number>} */
  async _components() {
    const paths = await this._templates(join(this._website, 'client/components'))
    const source = await readFile(join(this._website, 'client/shared/styles/tailwind.src.css'), 'utf8')
    await this._componentStyles(paths, StyleBuild._references(source))
    return paths.length
  }

  /** @param {string} source @returns {string} */
  static _references(source) {
    return source.replace('@import "tailwindcss/theme.css" layer(theme);', '@import "tailwindcss/theme.css" reference;').replace('@theme {', '@theme reference {').replace(/^@source .*;$/gm, '')
  }

  /** @param {string} folder @returns {Promise<string[]>} */
  async _templates(folder) {
    const entries = await readdir(folder, { withFileTypes: true })
    const paths = await Promise.all(entries.map((entry) => this._templateEntry(folder, entry)))
    return paths.flat().sort()
  }

  /** @param {string} folder @param {import('node:fs').Dirent} entry @returns {Promise<string[]>} */
  async _templateEntry(folder, entry) {
    const path = join(folder, entry.name)
    const folders = await this._directoryTemplates(entry, path)
    return [...folders, ...StyleBuild._templateFile(entry, path)]
  }

  /** @param {import('node:fs').Dirent} entry @param {string} path @returns {Promise<string[]>} */
  async _directoryTemplates(entry, path) {
    if (entry.isDirectory()) return this._templates(path)
    return []
  }

  /** @param {import('node:fs').Dirent} entry @param {string} path @returns {string[]} */
  static _templateFile(entry, path) {
    return entry.isFile() && entry.name === 'tac.html' ? [path] : []
  }

  /** @param {string[]} paths @param {string} source @returns {Promise<void>} */
  async _componentStyles(paths, source) {
    for (const path of paths) await this._component(path, source)
  }

  /** @param {string} path @param {string} source @returns {Promise<void>} */
  async _component(path, source) {
    const template = await readFile(path, 'utf8')
    const scanner = new this._Scanner({})
    const candidates = scanner.scanFiles([{ content: template, extension: 'html' }])
    const compiled = await this._compile(source, { base: dirname(path), onDependency: () => {}, customCssResolver: async (id) => this._require.resolve(id) })
    const styles = this._componentRules(compiled.build(candidates))
    const output = join(dirname(path), 'tac.css')
    const original = await readFile(output, 'utf8')
    const authored = StyleBuild._authored(original)
    await this._persist(output, `${authored}\n\n${StyleBuild._start}\n${styles.trim()}\n${StyleBuild._end}\n`, original)
  }

  /** @param {string} styles @returns {string} */
  _componentRules(styles) {
    const registrations = [...styles.matchAll(StyleBuild._properties)]
    this._recordRegistrations(registrations)
    return styles.replace(StyleBuild._properties, '')
  }

  /** @param {RegExpMatchArray[]} registrations @returns {void} */
  _recordRegistrations(registrations) {
    for (const registration of registrations) this._registrations.set(registration[0].split('{')[0].trim(), registration[0])
  }

  /** @param {string} original @returns {string} */
  static _authored(original) {
    const start = original.indexOf(StyleBuild._start)
    const end = original.indexOf(StyleBuild._end)
    return StyleBuild._outsideGenerated(original, start, end)
  }

  /** @param {string} original @param {number} start @param {number} end @returns {string} */
  static _outsideGenerated(original, start, end) {
    if (start === -1) return original.trimEnd()
    return StyleBuild._retained(original, start, end)
  }

  /** @param {string} original @param {number} start @param {number} end @returns {string} */
  static _retained(original, start, end) {
    StyleBuild._requireEnd(end)
    return `${original.slice(0, start)}${original.slice(end + StyleBuild._end.length)}`.trimEnd()
  }

  /** @param {number} end @returns {void} */
  static _requireEnd(end) {
    if (end === -1) throw new Error('Component stylesheet has an incomplete generated Tailwind region.')
  }

  /** @returns {Promise<void>} */
  async _foundation() {
    const registrations = [...this._registrations.entries()].sort(([left], [right]) => left.localeCompare(right)).map(([, value]) => value)
    const css = `/*! Tailwind CSS 4.3.3 variable registrations | MIT License | https://tailwindcss.com */\n${registrations.join('\n\n')}\n`
    const output = join(this._website, 'client/shared/styles/tailwind-properties.css')
    const original = await StyleBuild._existing(output)
    await this._persist(output, css, original)
  }

  /** @param {string} path @returns {Promise<string>} */
  static async _existing(path) {
    try {
      return await readFile(path, 'utf8')
    } catch (error) {
      return StyleBuild._missing(/** @type {NodeJS.ErrnoException} */ (error))
    }
  }

  /** @param {NodeJS.ErrnoException} error @returns {string} */
  static _missing(error) {
    if (error.code === 'ENOENT') return ''
    throw error
  }

  /** @param {string} path @param {string} expected @param {string} original @returns {Promise<void>} */
  async _persist(path, expected, original) {
    if (expected === original) return
    await this._changed(path, expected)
  }

  /** @param {string} path @param {string} expected @returns {Promise<void>} */
  async _changed(path, expected) {
    if (this._check) throw new Error(`Generated component styles are stale: ${path}`)
    await writeFile(path, expected)
  }
}

await StyleBuild.run()
