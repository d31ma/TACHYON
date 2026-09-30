// @ts-nocheck
import { CargoArtifacts } from '../../scripts/cargo-artifacts.mjs'
import { expect, test } from 'bun:test'
import { spawnSync } from 'node:child_process'
import { existsSync } from 'node:fs'
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { delimiter, dirname, join } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

class Locations {
  static repository = fileURLToPath(new URL('../../', import.meta.url))
  static dataRoot = new URL('../client/shared/data/', import.meta.url)
}

function isService(name) {
  return /^client\/services\/[A-Z]\w*Service\.(js|ts)$/.test(name)
}

function classNameOf(name) {
  return name.slice(name.lastIndexOf('/') + 1, name.lastIndexOf('.'))
}

function instanceNameOf(className) {
  const stem = className.replace(/Service$/, '')
  return stem[0].toLowerCase() + stem.slice(1)
}

function generatedPageFiles(classNames) {
  const placeholders = classNames.map((className) => `{${instanceNameOf(className)}}`)
  return [
    { name: 'client/pages/tac.html', code: '<example-view hydrate="load" />\n' },
    { name: 'client/components/example/view/tac.html', code: `<main>${placeholders.join(' ')}</main>\n` },
  ]
}

function findBinary(repository) {
  if (process.env.TAC_BIN !== undefined) return process.env.TAC_BIN
  const executable = process.platform === 'win32' ? 'ty.exe' : 'ty'
  const release = CargoArtifacts.path('release', executable)
  if (existsSync(release)) return release
  const debug = CargoArtifacts.path('debug', executable)
  if (existsSync(debug)) return debug
  return null
}

async function writeProject(project, files) {
  for (const file of files) {
    const destination = join(project, file.name)
    await mkdir(dirname(destination), { recursive: true })
    await writeFile(destination, file.code)
  }
}

function serviceExamplesFromFiles(label, files = []) {
  const examples = []
  for (const extension of ['js', 'ts']) {
    const services = files.filter(({ name }) => isService(name) && name.endsWith(`.${extension}`))
    if (services.length === 0) continue
    const sameLanguageCode = files.filter(({ name }) => name.endsWith(`.${extension}`)).map(({ code }) => code).join('\n')
    const isImported = (name) => sameLanguageCode.includes(`/${name.split('/').slice(1).join('/')}`)
    const isClientFileForLanguage = (name) => name.startsWith('client/')
      && (!/\.(js|ts|dart|swift|kt|cs|rs|go)$/.test(name) || name.endsWith(`.${extension}`) || isImported(name))
    const project = files.filter(({ name }) => isClientFileForLanguage(name))
    const classes = services.map(({ name }) => classNameOf(name))
    const hasPage = project.some(({ name }) => /^client\/pages\/.*tac\.html$/.test(name))
    const exampleFiles = hasPage ? project : [...generatedPageFiles(classes), ...project]
    examples.push({ name: `${label} [${extension}]`, classes, files: exampleFiles })
  }
  return examples
}

function serviceExamplesFromClass(label, code = '') {
  const className = code.match(/export default class ([A-Z]\w*Service)\b/)?.[1]
  if (!className) return []
  const serviceFile = { name: `client/services/${className}.js`, code }
  const noViewUsesSettings = { name: 'client/manifest.json', code: '{"tac":{"visibility":{"check":false}}}\n' }
  return [{ name: label, classes: [className], files: [...generatedPageFiles([className]), noViewUsesSettings, serviceFile] }]
}

function collectExamples(docs, catalogue) {
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
    const section = docs.topics[topic].sections.find((entry) => entry.heading === heading)
    examples.push(...serviceExamplesFromClass(`${topic}: ${section.heading}`, section.code))
    examples.push(...serviceExamplesFromFiles(`${topic}: ${section.heading}`, section.files))
  }
  const recipe = docs.topics.cookbook.sections.find((section) => section.heading === 'Signal-driven UI')
  examples.push(...serviceExamplesFromFiles('signal recipe', recipe.files))
  const mapping = docs.topics.annotations.sections.find((section) => section.heading === 'Mapping: Tac')
  examples.push(...serviceExamplesFromFiles('annotations: Mapping: Tac', mapping.files))
  for (const section of docs.topics['application-configuration'].sections) {
    examples.push(...serviceExamplesFromFiles('application configuration', section.files))
  }
  for (const feature of catalogue.features.filter(({ id }) => ['companions', 'persisted', 'fetch-cache'].includes(id))) {
    examples.push(...serviceExamplesFromFiles(feature.id, feature.files))
  }
  return examples
}

async function checkRuntimeContract(example, module) {
  if (example.name.startsWith('annotations: Mapping: Tac')) {
    expect(typeof module.__tachyon_create_owner__, example.name).toBe('function')
    const transport = async () => { throw new Error('a mapping sends no request') }
    const service = await module.__tachyon_create_owner__({}, new AbortController().signal, transport)
    await service.load()
    expect(service.title, example.name).toBe('product 7: Anvil')
  }
  if (example.name === 'calling-the-server: Client classes that call the server [js]') {
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
  const docs = JSON.parse(await readFile(new URL('docs.json', Locations.dataRoot), 'utf8'))
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
        const module = await import(pathToFileURL(join(project, 'dist/.tachyon/services', `${className}.js`)).href)
        expect(typeof module.default, `${example.name}: ${className}`).toBe('function')
        await checkRuntimeContract(example, module)
      }
    }
  } finally { await rm(root, { recursive: true, force: true }) }
}, 120000)
