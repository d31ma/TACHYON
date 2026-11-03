import { CargoArtifacts } from '../../scripts/cargo-artifacts.mjs'
import { expect, test } from 'bun:test'
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { delimiter, dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

/**
 * @typedef {{ name: string, code: string }} ExampleFile
 * @typedef {{ heading: string, code?: string, files?: ExampleFile[] }} DocsSection
 * @typedef {{ topics: Record<string, { sections: DocsSection[] }> }} DocsCatalogue
 * @typedef {{ features: { id: string, files?: ExampleFile[] }[] }} FeatureCatalogue
 * @typedef {{ name: string, classes: string[], files: ExampleFile[] }} Example
 * @typedef {{ load(): Promise<void>, title?: string, products?: unknown }} LoadedService
 * @typedef {object} ServiceModule
 * @property {new (client: { all(): Promise<unknown[]> }) => LoadedService} default
 * @property {(settings: object, lifecycle: AbortSignal, transport: () => Promise<never>) => Promise<LoadedService>} __tachyon_create_owner__
 */

class Locations {
  static repository = fileURLToPath(new URL('../../', import.meta.url))
  static dataRoot = new URL('../client/shared/data/', import.meta.url)
}

/** @param {string} name */
function isService(name) {
  return /^client\/services\/[A-Z]\w*Service\.(js|ts)$/.test(name)
}

/** @param {string} name */
function classNameOf(name) {
  return name.slice(name.lastIndexOf('/') + 1, name.lastIndexOf('.'))
}

/** @param {string} className */
function instanceNameOf(className) {
  const stem = className.replace(/Service$/, '')
  return stem[0].toLowerCase() + stem.slice(1)
}

/**
 * @param {string[]} classNames
 * @returns {ExampleFile[]}
 */
function generatedPageFiles(classNames) {
  const placeholders = classNames.map((className) => `{${instanceNameOf(className)}}`)
  return [
    { name: 'client/pages/tac.html', code: '<example-view hydrate="load" />\n' },
    { name: 'client/components/example/view/tac.html', code: `<main>${placeholders.join(' ')}</main>\n` },
  ]
}

/** @param {string} repository */
function findBinary(repository) {
  if (process.env.TAC_BIN !== undefined) return process.env.TAC_BIN
  const executable = process.platform === 'win32' ? 'ty.exe' : 'ty'
  const release = CargoArtifacts.path('release', executable)
  if (existsSync(release)) return release
  const debug = CargoArtifacts.path('debug', executable)
  if (existsSync(debug)) return debug
  return null
}

/**
 * @param {string} project
 * @param {ExampleFile[]} files
 */
async function writeProject(project, files) {
  for (const file of files) {
    const destination = join(project, file.name)
    await mkdir(dirname(destination), { recursive: true })
    await writeFile(destination, file.code)
  }
}

/**
 * @param {string} label
 * @param {ExampleFile[]} [files]
 * @returns {Example[]}
 */
function serviceExamplesFromFiles(label, files = []) {
  /** @type {Example[]} */
  const examples = []
  for (const extension of ['js', 'ts']) {
    const services = files.filter(({ name }) => isService(name) && name.endsWith(`.${extension}`))
    if (services.length === 0) continue
    const sameLanguageCode = files.filter(({ name }) => name.endsWith(`.${extension}`)).map(({ code }) => code).join('\n')
    const isImported = (/** @type {string} */ name) => sameLanguageCode.includes(`/${name.split('/').slice(1).join('/')}`)
    const isClientFileForLanguage = (/** @type {string} */ name) => name.startsWith('client/')
      && (!/\.(js|ts|dart|swift|kt|cs|rs|go)$/.test(name) || name.endsWith(`.${extension}`) || isImported(name))
    const project = files.filter(({ name }) => isClientFileForLanguage(name))
    const classes = services.map(({ name }) => classNameOf(name))
    const hasPage = project.some(({ name }) => /^client\/pages\/.*tac\.html$/.test(name))
    const exampleFiles = hasPage ? project : [...generatedPageFiles(classes), ...project]
    examples.push({ name: `${label} [${extension}]`, classes, files: exampleFiles })
  }
  return examples
}

/**
 * @param {string} label
 * @param {string} [code]
 * @returns {Example[]}
 */
function serviceExamplesFromClass(label, code = '') {
  const className = code.match(/export default class ([A-Z]\w*Service)\b/)?.[1]
  if (!className) return []
  const serviceFile = { name: `client/services/${className}.js`, code }
  const noViewUsesSettings = { name: 'client/manifest.json', code: '{"tac":{"visibility":{"check":false}}}\n' }
  return [{ name: label, classes: [className], files: [...generatedPageFiles([className]), noViewUsesSettings, serviceFile] }]
}

/**
 * @param {DocsCatalogue} docs
 * @param {string} topic
 * @param {string} heading
 * @returns {DocsSection}
 */
function sectionNamed(docs, topic, heading) {
  const section = docs.topics[topic].sections.find((entry) => entry.heading === heading)
  if (section === undefined) throw new Error(`docs topic '${topic}' has no section '${heading}'`)
  return section
}

/**
 * @param {DocsCatalogue} docs
 * @param {FeatureCatalogue} catalogue
 * @returns {Example[]}
 */
function collectExamples(docs, catalogue) {
  /** @type {Example[]} */
  const examples = []
  const browserSections = [
    ['behavior-annotations', 'Shared browser behavior annotations'],
    ['behavior-annotations', 'Create the behavior context in OnStart'],
    ['isolation-and-security', 'Security rules for browser code'],
  ]
  for (const topic of ['services', 'browser-storage', 'calling-the-server', 'server-data', 'topics', 'devices', 'preview-and-build', 'application-events']) {
    for (const section of docs.topics[topic].sections) browserSections.push([topic, section.heading])
  }
  for (const [topic, heading] of browserSections) {
    const section = sectionNamed(docs, topic, heading)
    examples.push(...serviceExamplesFromClass(`${topic}: ${section.heading}`, section.code))
    examples.push(...serviceExamplesFromFiles(`${topic}: ${section.heading}`, section.files))
  }
  const recipe = sectionNamed(docs, 'cookbook', 'Signal-driven UI')
  examples.push(...serviceExamplesFromFiles('signal recipe', recipe.files))
  const mapping = sectionNamed(docs, 'annotations', 'Mapping: Tac')
  examples.push(...serviceExamplesFromFiles('annotations: Mapping: Tac', mapping.files))
  for (const section of docs.topics['application-configuration'].sections) {
    examples.push(...serviceExamplesFromFiles('application configuration', section.files))
  }
  for (const feature of catalogue.features.filter(({ id }) => ['companions', 'persisted', 'fetch-cache'].includes(id))) {
    examples.push(...serviceExamplesFromFiles(feature.id, feature.files))
  }
  return examples
}

/**
 * @param {Example} example
 * @param {ServiceModule} module
 */
async function checkRuntimeContract(example, module) {
  if (example.name.startsWith('annotations: Mapping: Tac')) {
    expect(typeof module.__tachyon_create_owner__, example.name).toBe('function')
    const transport = async () => { throw new Error('a mapping sends no request') }
    const service = await module.__tachyon_create_owner__({}, new AbortController().signal, transport)
    await service.load()
    expect(service.title, example.name).toBe('product 7: Anvil')
  }
  if (example.name === 'calling-the-server: Client classes that call the server [js]') {
    /** @type {string[]} */
    const calls = []
    expect(typeof module.__tachyon_create_owner__).toBe('function')
    const service = new module.default({ all: async () => {
      calls.push('all')
      return [{ name: 'public product' }]
    } })
    await service.load()
    expect(calls).toEqual(['all'])
    expect(service.products).toEqual([{ name: 'public product' }])
  }
}

test('published browser service examples compile and use the documented runtime contract', async () => {
  /** @type {DocsCatalogue} */
  const docs = JSON.parse(await readFile(new URL('docs.json', Locations.dataRoot), 'utf8'))
  /** @type {FeatureCatalogue} */
  const catalogue = JSON.parse(await readFile(new URL('features.json', Locations.dataRoot), 'utf8'))
  const examples = collectExamples(docs, catalogue)
  expect(examples.length).toBeGreaterThan(10)

  const binary = findBinary(Locations.repository)
  const env = {
    ...process.env,
    RUSTUP_TOOLCHAIN: '1.97.1',
    PATH: `${join(Locations.repository, 'node_modules/.bin')}${delimiter}${process.env.PATH ?? ''}`,
  }
  const root = await mkdtemp(join(tmpdir(), 'tachyon-published-examples-'))
  try {
    for (const [index, example] of examples.entries()) {
      const project = join(root, `example-${index}`)
      await writeProject(project, example.files)
      const command = binary ? [binary, 'build', project]
        : ['cargo', 'run', '--locked', '--bin', 'ty', '--manifest-path', join(Locations.repository, 'Cargo.toml'), '--', 'build', project]
      const built = spawnSync(command[0], command.slice(1), { encoding: 'utf8', timeout: 120000, env })
      expect(built.status, `${example.name}\n${built.stderr || built.stdout}`).toBe(0)
      for (const className of example.classes) {
        /** @type {ServiceModule} */
        const module = await import(pathToFileURL(join(project, 'dist/.tachyon/services', `${className}.js`)).href)
        expect(typeof module.default, `${example.name}: ${className}`).toBe('function')
        await checkRuntimeContract(example, module)
      }
    }
  } finally { await rm(root, { recursive: true, force: true }) }
}, 120000)
