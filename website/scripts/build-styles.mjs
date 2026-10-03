import { readFile, writeFile, readdir } from 'node:fs/promises'
import { createRequire } from 'node:module'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

export class StyleBuild {
  static #start = '/* @tachyon-generated-tailwind:start */'
  static #end = '/* @tachyon-generated-tailwind:end */'
  static #properties = /@property\s+--[\w-]+\s*\{[^}]*\}/g

  /** @type {string} */
  #website
  /** @type {boolean} */
  #check
  /** @type {typeof import('@tailwindcss/node').compile} */
  #compile
  /** @type {typeof import('@tailwindcss/oxide').Scanner} */
  #Scanner
  /** @type {NodeJS.Require} */
  #require
  /** @type {Map<string, string>} */
  #registrations = new Map()

  /** @param {boolean} check */
  constructor(check) {
    this.#website = fileURLToPath(new URL('../', import.meta.url))
    this.#check = check
    this.#require = StyleBuild.#dependencyRequire()
    this.#compile = this.#require('@tailwindcss/node').compile
    this.#Scanner = this.#require('@tailwindcss/oxide').Scanner
  }

  /** @returns {Promise<void>} */
  static async run() {
    const build = new StyleBuild(process.argv.includes('--check'))
    const started = performance.now()
    const count = await build.#components()
    await build.#foundation()
    process.stdout.write(`${JSON.stringify({ components: count, registrations: build.#registrations.size, check: build.#check, seconds: (performance.now() - started) / 1000 })}\n`)
  }

  /** @returns {NodeJS.Require} */
  static #dependencyRequire() {
    return createRequire(StyleBuild.#dependencyLocation())
  }

  /** @returns {string} */
  static #dependencyLocation() {
    if (process.env.TACHYON_WEBSITE_DEPENDENCIES) return resolve(process.env.TACHYON_WEBSITE_DEPENDENCIES, '../package.json')
    return fileURLToPath(import.meta.url)
  }

  /** @returns {Promise<number>} */
  async #components() {
    const paths = await this.#templates(join(this.#website, 'client/components'))
    const source = await readFile(join(this.#website, 'client/shared/styles/tailwind.src.css'), 'utf8')
    await this.#componentStyles(paths, StyleBuild.#references(source))
    return paths.length
  }

  /** @param {string} source @returns {string} */
  static #references(source) {
    return source.replace('@import "tailwindcss/theme.css" layer(theme);', '@import "tailwindcss/theme.css" reference;').replace('@theme {', '@theme reference {').replace(/^@source .*;$/gm, '')
  }

  /** @param {string} folder @returns {Promise<string[]>} */
  async #templates(folder) {
    const entries = await readdir(folder, { withFileTypes: true })
    const paths = await Promise.all(entries.map((entry) => this.#templateEntry(folder, entry)))
    return paths.flat().sort()
  }

  /** @param {string} folder @param {import('node:fs').Dirent} entry @returns {Promise<string[]>} */
  async #templateEntry(folder, entry) {
    const path = join(folder, entry.name)
    const folders = await this.#directoryTemplates(entry, path)
    return [...folders, ...StyleBuild.#templateFile(entry, path)]
  }

  /** @param {import('node:fs').Dirent} entry @param {string} path @returns {Promise<string[]>} */
  async #directoryTemplates(entry, path) {
    if (entry.isDirectory()) return this.#templates(path)
    return []
  }

  /** @param {import('node:fs').Dirent} entry @param {string} path @returns {string[]} */
  static #templateFile(entry, path) {
    return entry.isFile() && entry.name === 'tac.html' ? [path] : []
  }

  /** @param {string[]} paths @param {string} source @returns {Promise<void>} */
  async #componentStyles(paths, source) {
    for (const path of paths) await this.#component(path, source)
  }

  /** @param {string} path @param {string} source @returns {Promise<void>} */
  async #component(path, source) {
    const template = await readFile(path, 'utf8')
    const scanner = new this.#Scanner({})
    const candidates = scanner.scanFiles([{ content: template, extension: 'html' }])
    const compiled = await this.#compile(source, { base: dirname(path), onDependency: () => {}, customCssResolver: async (id) => this.#require.resolve(id) })
    const styles = this.#componentRules(compiled.build(candidates))
    const output = join(dirname(path), 'tac.css')
    const original = await readFile(output, 'utf8')
    const authored = StyleBuild.#authored(original)
    await this.#persist(output, `${authored}\n\n${StyleBuild.#start}\n${styles.trim()}\n${StyleBuild.#end}\n`, original)
  }

  /** @param {string} styles @returns {string} */
  #componentRules(styles) {
    const registrations = [...styles.matchAll(StyleBuild.#properties)]
    this.#recordRegistrations(registrations)
    return styles.replace(StyleBuild.#properties, '')
  }

  /** @param {RegExpMatchArray[]} registrations @returns {void} */
  #recordRegistrations(registrations) {
    for (const registration of registrations) this.#registrations.set(registration[0].split('{')[0].trim(), registration[0])
  }

  /** @param {string} original @returns {string} */
  static #authored(original) {
    const start = original.indexOf(StyleBuild.#start)
    const end = original.indexOf(StyleBuild.#end)
    return StyleBuild.#outsideGenerated(original, start, end)
  }

  /** @param {string} original @param {number} start @param {number} end @returns {string} */
  static #outsideGenerated(original, start, end) {
    if (start === -1) return original.trimEnd()
    return StyleBuild.#retained(original, start, end)
  }

  /** @param {string} original @param {number} start @param {number} end @returns {string} */
  static #retained(original, start, end) {
    StyleBuild.#requireEnd(end)
    return `${original.slice(0, start)}${original.slice(end + StyleBuild.#end.length)}`.trimEnd()
  }

  /** @param {number} end @returns {void} */
  static #requireEnd(end) {
    if (end === -1) throw new Error('Component stylesheet has an incomplete generated Tailwind region.')
  }

  /** @returns {Promise<void>} */
  async #foundation() {
    const registrations = [...this.#registrations.entries()].sort(([left], [right]) => left.localeCompare(right)).map(([, value]) => value)
    const css = `/*! Tailwind CSS 4.3.3 variable registrations | MIT License | https://tailwindcss.com */\n${registrations.join('\n\n')}\n`
    const output = join(this.#website, 'client/shared/styles/tailwind-properties.css')
    const original = await StyleBuild.#existing(output)
    await this.#persist(output, css, original)
  }

  /** @param {string} path @returns {Promise<string>} */
  static async #existing(path) {
    try {
      return await readFile(path, 'utf8')
    } catch (error) {
      return StyleBuild.#missing(/** @type {NodeJS.ErrnoException} */ (error))
    }
  }

  /** @param {NodeJS.ErrnoException} error @returns {string} */
  static #missing(error) {
    if (error.code === 'ENOENT') return ''
    throw error
  }

  /** @param {string} path @param {string} expected @param {string} original @returns {Promise<void>} */
  async #persist(path, expected, original) {
    if (expected === original) return
    await this.#changed(path, expected)
  }

  /** @param {string} path @param {string} expected @returns {Promise<void>} */
  async #changed(path, expected) {
    if (this.#check) throw new Error(`Generated component styles are stale: ${path}`)
    await writeFile(path, expected)
  }
}

await StyleBuild.run()
