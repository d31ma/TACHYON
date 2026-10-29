/** @import { SnippetFile } from '/models/SnippetFile.ts' */
/** @import { Section } from '/models/Section.ts' */
/** @import { LanguageEntry } from '/models/LanguageEntry.ts' */
/**
 * @typedef {Section & { files: SnippetFile[] }} DocsSection
 * @typedef {{ title: string, summary: string, sections: DocsSection[] }} DocsGuide
 * @typedef {{ order: string[], topics: Record<string, DocsGuide> }} DocsCatalogue
 * @typedef {{ id: string, title: string, summary: string, group: string, files: SnippetFile[] }} Feature
 * @typedef {{ groups: string[], features: Feature[] }} Features
 * @typedef {{ tac: { files: SnippetFile[] }, yon: { entries: LanguageEntry[] } }} Languages
 * @typedef {{ title: string, links: { label: string, href: string }[] }} FooterColumn
 * @typedef {{ name: string, required?: string[], scope?: string }} BuiltService
 * @typedef {{ k: string, name: string, services: BuiltService[], properties: { name: string }[] }} BuiltComponent
 * @typedef {{ name: string, resources: { name: string }[], post_construct?: unknown }} PlannedService
 * @typedef {{ path: string, name: string }} ListedFile
 */
import { SiteArtifacts } from '../../scripts/site-artifacts.mjs'
import { beforeAll, describe, expect, test } from 'bun:test'
import { existsSync } from 'node:fs'
import { readdir, readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { ensureBundle } from './helpers/ensure-bundle.js'

class Patterns {
  static needs = /[@#[<]+Needs(\(\))?[\s\]>]|Annotated\[[^\]]*\bneeds\b/
}

class Found {
  /** @template T @param {T | null | undefined} value @returns {T} */
  static required(value) {
    if (value === undefined || value === null) throw new Error('A required value is missing')
    return value
  }
}

class Snippets {
  /** @param {...string} names @returns {SnippetFile[]} */
  static named(...names) {
    return /** @type {SnippetFile[]} */ (names.map((name) => ({ name })))
  }
}

class SiteData {
  /** @returns {Promise<DocsCatalogue>} */
  static docs() {
    return readJson('client/shared/data/docs.json')
  }

  /** @returns {Promise<Features>} */
  static features() {
    return readJson('client/shared/data/features.json')
  }

  /** @returns {Promise<Languages>} */
  static languages() {
    return readJson('client/shared/data/languages.json')
  }

  /** @returns {Promise<FooterColumn[]>} */
  static footer() {
    return readJson('client/shared/data/footer.json')
  }
}

/** @param {string} relative @returns {Promise<string>} */
function read(relative) {
  if (relative.startsWith('dist/web/')) {
    const project = fileURLToPath(new URL('..', import.meta.url))
    return readFile(`${SiteArtifacts.web(project)}/${relative.slice(9)}`, 'utf8')
  }
  return readFile(new URL(relative, new URL('..', import.meta.url)), 'utf8')
}

/** @param {string} relative */
async function readJson(relative) {
  return JSON.parse(await read(relative))
}

/** @param {string} directory @returns {Promise<ListedFile[]>} */
async function listFiles(directory) {
  /** @type {ListedFile[]} */
  const files = []
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const child = `${directory}/${entry.name}`
    if (entry.isDirectory()) files.push(...await listFiles(child))
    else files.push({ path: child, name: entry.name })
  }
  return files
}

/** @param {string} className */
function instanceNameOf(className) {
  const stem = className.slice(0, -'Service'.length)
  return stem[0].toLowerCase() + stem.slice(1)
}

class Built {
  static routes = ['dist/web/index.html', 'dist/web/docs/index.html', 'dist/web/docs/_topic/index.html',
    'dist/web/docs/features/index.html', 'dist/web/docs/features/_id/index.html']

  /** @param {string} route */
  static async components(route) {
    const html = await read(route)
    const view = JSON.parse(Found.required(html.match(/<script id="tachyon-view"[^>]*>([\s\S]*?)<\/script>/))[1])
    /** @type {BuiltComponent[]} */
    const components = []
    const pending = [view]
    while (pending.length > 0) {
      const node = pending.pop()
      if (Array.isArray(node)) pending.push(...node)
      else if (node && typeof node === 'object') {
        pending.push(...Object.values(node))
        if (node.k === 'component') components.push(node)
      }
    }
    return components
  }

  /** @param {string} route @param {string} component @param {string} name */
  static async service(route, component, name) {
    const found = Found.required((await Built.components(route)).find((node) => node.name === component))
    return found.services.find((service) => service.name === name)
  }

  /** @param {string} className */
  static async plan(className) {
    const module = await read(`dist/web/.tachyon/services/${className}.js`)
    /** @type {{ types: PlannedService[] }} */
    const plan = JSON.parse(Found.required(module.match(/^export const __tachyon_event_plan__ = (\{.*\});?$/m))[1])
    return Found.required(plan.types.find((type) => type.name === className))
  }
}

beforeAll(ensureBundle, 300_000)

describe('published output', () => {
  test('every route links the declared stylesheet, script and the client runtime', async () => {
    for (const route of ['dist/web/index.html', 'dist/web/docs/index.html', 'dist/web/docs/_topic/index.html']) {
      const html = await read(route)
      expect(html).toContain('/shared/styles/site.css')
      expect(html).toContain('/shared/assets/duvay/duvay-wc-site.min.js')
      expect(html).not.toContain('/shared/assets/duvay/duvay-wc.min.js')
      expect(html.indexOf('/shared/scripts/ThemeDefault.js')).toBeGreaterThan(-1)
      expect(html.indexOf('/shared/scripts/ThemeDefault.js')).toBeLessThan(html.indexOf('/shared/assets/duvay/duvay.min.js'))
      expect(html).not.toContain('/shared/scripts/imports.js')
      expect(html).toContain('/.tachyon/tac-client.js')
      expect(html).not.toContain('"module":"/client.js"')
    }
  })

  test('the shared stylesheet orders DuVay below Tailwind', async () => {
    const css = await read('client/shared/styles/site.css')
    expect(css).toContain('@layer duvay, theme, base, components, brand, utilities;')
    expect(css).toContain('layer(duvay)')
  })

  test('Tailwind is imported without Preflight', async () => {
    const source = await read('client/shared/styles/tailwind.src.css')
    expect(source).toContain('tailwindcss/utilities.css')
    expect(source).not.toContain('"tailwindcss"')
  })

  test('every property a view hands to a component reaches that component service', async () => {
    const routes = ['dist/web/docs/index.html', 'dist/web/docs/_topic/index.html', 'dist/web/docs/features/_id/index.html']
    const lost = []
    for (const route of routes) {
      for (const node of await Built.components(route)) {
        const receivers = node.services.filter(service => Array.isArray(service.required))
        if (receivers.length === 0) continue
        const required = new Set(receivers.flatMap(service => service.required))
        for (const property of node.properties ?? []) {
          if (!required.has(property.name)) lost.push(`${route}: <${node.name} :${property.name}>`)
        }
      }
    }
    expect(lost).toEqual([])
  })

  test('interactive services inject owner-scoped render resources', async () => {
    for (const className of ['ClipboardService', 'SearchService']) {
      const resources = (await Built.plan(className)).resources.map((resource) => resource.name)
      expect(resources).toContain('taskScheduler')
      expect(resources).toContain('lifecycle')
    }
  })

  test('live site services start in their OnStart method', async () => {
    const manifest = JSON.parse(await read('client/manifest.json'))
    expect(manifest.tac.metadata.routes['/']).toBeDefined()
    for (const className of ['FeatureService', 'TopicService', 'SearchService']) {
      expect((await Built.plan(className)).post_construct, className).toBeTruthy()
    }
  })

  test('the feature page hands its route and catalogue to the feature service', async () => {
    const route = 'dist/web/docs/features/_id/index.html'
    const feature = Found.required((await Built.components(route)).find((node) => node.name === 'docs-feature'))
    expect(feature.properties.map((property) => property.name)).toEqual(expect.arrayContaining(['route', 'catalogue']))
    const service = Found.required(await Built.service(route, 'docs-feature', 'feature'))
    expect(service.required).toEqual(expect.arrayContaining(['route', 'catalogue']))
    const manifest = JSON.parse(await read('client/manifest.json'))
    expect(manifest.tac.browserApis).not.toContain('documentTitle')
  })

  test('a component is a view and its style; behaviour is a service', async () => {
    const components = fileURLToPath(new URL('../client/components', import.meta.url))
    const views = []
    for (const file of await listFiles(components)) {
      expect(['tac.html', 'tac.css'], file.path).toContain(file.name)
      if (file.name === 'tac.html') views.push(file.path)
    }
    expect(views.length).toBeGreaterThan(15)

    const services = (await readdir(fileURLToPath(new URL('../client/services', import.meta.url))))
    expect(services.length).toBeGreaterThanOrEqual(15)
    const built = await readdir(`${SiteArtifacts.web(fileURLToPath(new URL('..', import.meta.url)))}/.tachyon/services`)
    const instances = new Set()
    for (const file of services) {
      const [, className] = file.match(/^([A-Z]\w*Service)\.(js|ts|py|dart|rs|cs|fs|go|rb|kt|swift|lua|java)$/) ?? []
      expect(className, file).toBeDefined()
      expect(built, file).toContain(`${className}.js`)
      instances.add(instanceNameOf(Found.required(className)))
    }
    const shown = new Map()
    for (const route of Built.routes) {
      for (const node of await Built.components(route)) {
        for (const service of node.services) shown.set(service.name, service)
      }
    }
    expect([...instances].sort()).toEqual([...shown.keys()].sort())
    for (const name of ['snippet', 'toc', 'pager', 'pagelinks']) {
      expect(Array.isArray(shown.get(name).required), name).toBe(true)
      expect(shown.get(name).scope, name).toBe('prototype')
    }
    expect(shown.get('clipboard').scope).toBe('prototype')
  })
})

describe('language coverage', () => {
  test('application events cover every owned language without retired runtime calls', async () => {
    const docs = await SiteData.docs()
    const topic = docs.topics['application-events']
    const browser = Found.required(topic.sections.find(section => section.heading.startsWith('Browser services')))
    const compiled = Found.required(topic.sections.find(section => section.files?.some(file => file.name === 'client/services/CatalogService.swift')))
    const views = /^client\/(models|pages|components)\//
    /** @param {DocsSection} section */
    const services = (section) => section.files.filter(file => !views.test(file.name))
    for (const file of [...services(browser), ...services(compiled)]) expect(file.name).toMatch(/^client\/services\/CatalogService\.[a-z]+$/)
    const yon = Found.required(topic.sections.find(section => section.heading.startsWith('Yon:')))
    expect(services(browser).map(file => file.name.split('.').pop()).sort()).toEqual(['js', 'ts'])
    expect(services(compiled).map(file => file.name.split('.').pop()).sort()).toEqual(['cs', 'dart', 'fs', 'go', 'hpp', 'java', 'kt', 'rb', 'rs', 'swift'])
    for (const file of compiled.files.filter(file => file.name.startsWith('client/models/'))) expect(file.name).toMatch(/^client\/models\/CatalogCount\.[a-z]+$/)
    expect(yon.files.filter(file => /\/controllers\//.test(file.name)).map(file => file.name.split('.').pop()).sort())
      .toEqual(['cs', 'dart', 'fs', 'go', 'groovy', 'hpp', 'java', 'js', 'kt', 'lua', 'php', 'py', 'rb', 'rs', 'ts'])
    for (const section of topic.sections) for (const file of section.files ?? []) {
      expect(file.code).not.toContain('this.tac')
      expect(file.code).not.toMatch(/tac_publish|tacPublish|Tac\.Publish|\bmount\(/)
    }
  })

  test('strict role layouts have complete annotation-discovered examples in every supported language', async () => {
    const docs = await SiteData.docs()
    const sections = docs.topics['project-folders'].sections
    /** @type {[string, string[], RegExp, RegExp][]} */
    const layouts = [
      ['yon', ['cs', 'dart', 'fs', 'go', 'groovy', 'hpp', 'java', 'js', 'kt', 'lua', 'php', 'py', 'rb', 'rs', 'ts'], /^server\/controllers\/MoneyController\.[a-z]+$/, /^server\/services\/MoneyService\./],
      ['tac', ['cs', 'dart', 'fs', 'go', 'hpp', 'java', 'js', 'kt', 'lua', 'py', 'rb', 'rs', 'swift', 'ts'], /^client\/services\/MoneyService\.[a-z]+$/, /^client\/entities\/MoneyEntity\./],
    ]
    for (const [framework, extensions, entryPattern, modulePattern] of layouts) {
      const section = Found.required(sections.find(entry => entry.files?.some(file => entryPattern.test(file.name))))
      expect(section.axis).toBe('language')
      expect(section.files.some(file => file.name.endsWith('.modules.json'))).toBe(false)
      const entries = section.files.filter(file => entryPattern.test(file.name))
      expect(entries.map(file => file.name.split('.').pop()).sort()).toEqual(extensions)
      if (framework === 'tac') {
        expect(section.files.some(file => file.name === 'client/pages/tac.html' && file.code.includes('<money-panel'))).toBe(true)
        expect(Found.required(section.files.find(file => file.name === 'client/components/money/panel/tac.html')).code).toContain('{money.amount}')
      }
      for (const extension of extensions) {
        const entry = entries.find(file => file.name.endsWith(`.${extension}`))
        const modules = section.files.filter(file => file !== entry && file.name.endsWith(`.${extension}`) && !file.name.includes('/models/'))
        expect(modules.length).toBe(1)
        for (const module of modules) {
          expect(module.code.length).toBeGreaterThan(0)
          expect(module.name).toMatch(modulePattern)
        }
        if (framework === 'yon') {
          const controller = Found.required(section.files.find(file => file.name.endsWith(`/MoneyController.${extension}`))).code
          expect(controller).toMatch(Patterns.needs)
          expect(controller).not.toContain('YonResponse')
          const selected = modules.map(file => file.code).join('\n')
          expect(selected).not.toMatch(/[@\[](?:yon::)?[Ii]nject\b/)
          for (const module of modules) {
            const authored = /\bconstructor\s*\(|\b__init__\s*\(|\b__construct\s*\(|\bfn\s+new\s*\(/
            if (Patterns.needs.test(module.code)) expect(module.code).not.toMatch(authored)
            else if (extension !== 'rs') expect(module.code).not.toMatch(authored)
          }
          expect(selected).not.toMatch(/(?<!type )\bMoney(?:Controller|Service)\s*\(/)
        }
      }
    }
  })

  test('Tac configuration binding declares the same complete prefix in every language', async () => {
    const docs = await SiteData.docs()
    const section = Found.required(docs.topics.annotations.sections.find(section => section.heading === 'Configuration binding: Tac'))
    const manifest = JSON.parse(Found.required(section.files.find(file => file.name === 'client/manifest.json')).code)
    expect(manifest.tac.catalog).toEqual({ title: 'Catalog', page: { size: 20 }, cache: { enabled: true } })
    const configurations = section.files.filter(file => file.name.startsWith('client/setups/'))
    expect(configurations.map(file => file.name.split('.').pop()).sort()).toEqual(['cs', 'dart', 'fs', 'go', 'hpp', 'java', 'js', 'kt', 'lua', 'py', 'rb', 'rs', 'swift', 'ts'])
    const settingsModel = Found.required(section.files.find(file => file.name === 'client/models/CatalogSettings.hpp'))
    for (const file of configurations) {
      const extension = file.name.split('.').pop()
      const bound = extension === 'hpp' ? settingsModel : file
      const policy = JSON.parse(Found.required(bound.code.match(/Settings\((\{[^\n]*\})\)/))[1])
      expect(policy.prefix).toBe('TAC_CATALOG_')
      expect(policy.fields).toBeUndefined()
      const fields = extension === 'rb' || extension === 'lua' ? ['page_size', 'cache_enabled'] : ['pageSize', 'cacheEnabled']
      const limits = extension === 'py' ? ['min(1)', 'max(100)'] : ['Min(1)', 'Max(100)']
      for (const field of [...fields, 'title', ...limits]) expect(bound.code).toContain(field)
    }
    const go = Found.required(section.files.find(file => file.name === 'client/services/SettingsService.go')).code
    expect(go).toContain('BehaviorsProperties[CatalogSetup](published)')
    expect(go).not.toContain('catalogProperties')
  })

  test('application configuration offers independently runnable examples in every Yon language', async () => {
    const docs = await SiteData.docs()
    const section = Found.required(docs.topics['application-configuration'].sections
      .find((entry) => entry.heading === 'Access from every Yon language'))
    expect(section).toBeDefined()
    expect(section.axis).toBe('language')
    const handlers = section.files.filter((file) => /^server\/controllers\//.test(file.name))
    expect(handlers.map((file) => file.name.split('.').pop()).sort())
      .toEqual(['cs', 'dart', 'fs', 'go', 'groovy', 'hpp', 'java', 'js', 'kt', 'lua', 'php', 'py', 'rb', 'rs', 'ts'])
    for (const handler of handlers) {
      expect(handler.name).toMatch(/^server\/controllers\/ConfigurationController\./)
      expect(handler.code).toMatch(/RequestRoute|request_route/)
      const extension = handler.name.split('.').pop()
      const controller = Found.required(section.files.find((file) => file.name.endsWith(`/ConfigurationController.${extension}`)))
      const settings = Found.required(section.files.find((file) => file.name === `server/models/CatalogSettings.${extension}`))
      const endpoint = Found.required(section.files.find((file) => file.name === `server/models/Endpoint.${extension}`))
      expect(settings.code).toMatch(/[@[<]Settings\(|@settings\(/)
      expect(endpoint.code).toMatch(/\bSetting\b|\bsetting\b/)
      expect(controller.code).toMatch(/GetRoute|get_route/)
    }
    const shared = section.files.filter((file) => file.name.endsWith('.json'))
    expect(shared.map((file) => file.name)).toEqual(['server/application.json'])
    expect(JSON.parse(shared[0].code)).toEqual({
      server: { host: '127.0.0.1', port: 8080 },
      catalog: { page: { size: 20 } },
    })
    const topicTemplate = await read('client/components/docs/topic/tac.html')
    expect(topicTemplate).toContain(':axis="section.axis')
  })

  test('a docs section shows its summary as a lede before its body', async () => {
    const topicTemplate = await read('client/components/docs/topic/tac.html')
    const lede = topicTemplate.indexOf('inlineCode.parts(section.summary)')
    expect(lede).toBeGreaterThan(-1)
    expect(lede).toBeLessThan(topicTemplate.indexOf('block of section.blocks'))
  })

  test('storage documents managed resources in every Yon language and browser Client discovery', async () => {
    const docs = await SiteData.docs()
    const repositories = Found.required(docs.topics['server-data'].sections.find(section => section.heading === 'DataSource in a service, in every Yon language'))
    expect(repositories.axis).toBe('language')
    const extensions = ['cs', 'dart', 'go', 'groovy', 'java', 'js', 'kt', 'php', 'py', 'rb', 'rs', 'ts']
    expect(repositories.files.filter(file => /\/controllers\//.test(file.name)).map(file => file.name.split('.').pop()).sort()).toEqual([...extensions, 'fs'].sort())
    const fsharp = repositories.files.filter(file => file.name.endsWith('.fs'))
    expect(fsharp.map(file => file.name)).toEqual(['server/services/CatalogService.fs', 'server/controllers/CatalogController.fs'])
    expect(fsharp[0].code).toContain('[<Service>]\ntype CatalogService(dataSource: DataSource) =')
    for (const extension of extensions) {
      const files = repositories.files.filter(file => file.name.endsWith(`.${extension}`))
      expect(files.some(file => file.name.endsWith(`/services/CatalogService.${extension}`))).toBe(true)
      expect(files.some(file => file.name.endsWith(`/controllers/CatalogController.${extension}`))).toBe(true)
      expect(files.map(file => file.code).join('\n')).toMatch(/DataSource|dataSource/)
      expect(files.map(file => file.code).join('\n')).toMatch(Patterns.needs)
    }
    const clients = Found.required(docs.topics['calling-the-server'].sections.find(section => section.heading === 'Client classes that call the server'))
    expect(clients.body).toContain('HTTPClient')
    expect(clients.body).toContain('explicitly configured')
    for (const extension of ['js', 'ts']) {
      const provider = Found.required(clients.files.find(file => file.name.endsWith(`/remotes/CatalogClient.${extension}`)))
      expect(provider.code).toMatch(/^@HTTPClient\b/m)
      expect(provider.code).toContain('@GetRequest(')
      const service = Found.required(clients.files.find(file => file.name === `client/services/CatalogService.${extension}`))
      expect(service.code).toMatch(Patterns.needs)
      expect(service.code).toContain('export default class CatalogService')
      expect(service.code).not.toMatch(/\bconstructor\s*\(|@Needs\(|\bfetch\s*\(/)
    }
    expect(Found.required(clients.files.find(file => file.name === 'client/components/catalog/page/tac.html')).code).toContain('catalog.load()')
    expect(Found.required(clients.files.find(file => file.name === 'client/pages/tac.html')).code).toContain('<catalog-page')
    expect(JSON.stringify(docs)).not.toMatch(/\.modules?\.json/)
  })

  test('first-app steps call the server through a client and test the service with a fake', async () => {
    const docs = await SiteData.docs()
    const sections = docs.topics['first-app'].sections
    const step4 = Found.required(sections.find(section => section.heading.startsWith('Step 4:')))
    expect(step4.axis).toBe('language')
    /** @param {string} name */
    const source = name => Found.required(step4.files.find(file => file.name === name)).code
    for (const extension of ['js', 'ts', 'dart']) {
      expect(source(`client/remotes/GreetingClient.${extension}`)).toContain('@GetRequest(')
      expect(source(`client/services/CounterService.${extension}`)).toContain('@Needs')
      expect(source(`client/services/CounterService.${extension}`)).not.toMatch(/\bfetch\s*\(/)
    }
    expect(step4.body).toContain('Credentials are omitted')
    expect(step4.body).toContain('same-origin only')
    const step5 = Found.required(sections.find(section => section.heading.startsWith('Step 5:')))
    const names = step5.files.map(file => file.name)
    expect(names).toEqual(expect.arrayContaining(['client/tests/CounterServiceTest.dart', 'client/tests/HelloTestSetup.dart', 'server/tests/GreetingControllerTest.dart']))
    expect(Found.required(step5.files.find(file => file.name === 'client/tests/CounterServiceTest.dart')).code).toContain('@Fake')
    expect(step5.body).toContain('without a network request')
  })

  test('application HTTP recipes use an HTTPClient, and cached reads use Cached on the client', async () => {
    const docs = await SiteData.docs()
    for (const section of [docs.topics.cookbook.sections[1], Found.required(docs.topics['application-configuration'].sections.find(section => section.heading.startsWith('Public Tac output')))]) {
      expect(section.axis).toBe('language')
      expect(section.files.some(file => /\/remotes\/\w+Client\.js$/.test(file.name))).toBe(true)
      expect(section.files.some(file => /\/remotes\/\w+Client\.ts$/.test(file.name))).toBe(true)
      expect(section.files.map(file => file.code).join('\n')).not.toMatch(/\bfetch\s*\(/)
    }
    const features = (await SiteData.features()).features
    for (const feature of features) {
      const code = (feature.files || []).map(file => file.code).join('\n')
      expect(code).not.toMatch(/\bfetch\s*\(/)
    }
    const worker = Found.required(features.find(feature => feature.id === 'fetch-cache'))
    expect(worker.summary).toContain('TY1361')
    expect(worker.summary).toContain('public read')
    for (const extension of ['js', 'ts']) {
      const client = Found.required(worker.files.find(file => file.name === `client/remotes/ProductsClient.${extension}`))
      expect(client.code).toContain('@Cached')
      expect(client.code).toContain('@ClearCache({"method":"list"})')
      const service = Found.required(worker.files.find(file => file.name === `client/services/CatalogService.${extension}`))
      expect(service.code).toContain('this.productsClient.list()')
      expect(service.code).toContain('this.productsClient.save(')
    }
  })

  test('application examples separate secrets, and Tac configuration lives in the web app manifest', async () => {
    const docs = await SiteData.docs()
    const topic = docs.topics['application-configuration']
    const files = topic.sections[0].files
    const application = JSON.parse(Found.required(files.find(file => file.name === 'server/application.json')).code)
    expect(application.yon).toBeUndefined()
    expect(application.tac).toBeUndefined()
    expect(application.server).toBeDefined()
    expect(JSON.stringify(application)).not.toMatch(/password|signing|\$\{/)
    const manifest = JSON.parse(Found.required(files.find(file => file.name === 'client/manifest.json')).code)
    expect(manifest.start_url).toBe('/')
    expect(manifest.tac).toEqual({ api: { url: '/api' } })
    expect(Found.required(files.find(file => file.name === '.env')).code).toContain('YON_DATASOURCE_PASSWORD=')
    expect(Found.required(files.find(file => file.name === '.gitignore')).code).toContain('.env')
    const published = JSON.parse(await read('dist/web/manifest.webmanifest'))
    expect(published).toEqual(JSON.parse(await read('client/manifest.json')))
    expect(published.yon).toBeUndefined()
    expect(existsSync(`${SiteArtifacts.web(fileURLToPath(new URL('..', import.meta.url)))}/.tachyon/application.json`)).toBe(false)
  })

  test('the Swift configuration example keeps native Tac separate from private Yon configuration', async () => {
    const docs = await SiteData.docs()
    const section = Found.required(docs.topics['application-configuration'].sections
      .find((entry) => entry.files?.some((file) => file.name.endsWith('Service.swift'))))
    expect(section).toBeDefined()
    const prose = JSON.stringify({ heading: section.heading, body: section.body })
    expect(prose).toMatch(/Swift/)
    expect(prose).toMatch(/native Tac/i)
    expect(prose).toMatch(/public/i)
    expect(prose).toMatch(/private/i)
    expect(prose).toMatch(/Yon/)
    expect(section.files.some((file) => /\/yon\./.test(file.name))).toBe(false)
    const swift = Found.required(section.files.find((file) => file.name.endsWith('Service.swift')))
    expect(swift.name).toBe('client/services/SettingsService.swift')
    expect(Found.required(section.files.find((file) => file.name.endsWith('/tac.html') && file.name.startsWith('client/components/'))).code).toContain('{settings.')
    expect(swift.code).toContain('PublicValues')
    expect(swift.code).not.toContain('YON_CATALOG_PAGE_SIZE')
  })

  test('every language Tachyon supports has a snippet in both layers', async () => {
    const languages = await SiteData.languages()
    const tac = languages.tac.files.map((file) => file.name.split('.').pop())
    const yon = languages.yon.entries.map((entry) => entry.id)

    for (const extension of ['js', 'ts', 'py', 'dart', 'rs', 'swift', 'kt', 'cs', 'fs', 'go', 'rb', 'lua', 'hpp']) {
      expect(tac).toContain(extension)
    }
    for (const extension of ['html', 'css']) {
      expect(tac).toContain(extension)
    }
    for (const id of ['javascript', 'typescript', 'python', 'java', 'groovy', 'csharp', 'fsharp',
                      'kotlin', 'php', 'rust', 'dart', 'go', 'ruby', 'lua', 'cpp']) {
      expect(yon).toContain(id)
    }
    expect(yon).toHaveLength(15)
    for (const file of languages.tac.files) {
      expect(file.code.length).toBeGreaterThan(20)
      expect(file.name).toMatch(/^client\/manifest\.json$|^client\/components\/basket\/tally\/tac\.(html|css)$|^client\/services\/TallyService\.[a-z]+$/)
    }
    expect(Found.required(languages.tac.files.find((file) => file.name.endsWith('/tac.html'))).code).toContain('{tally.')
    for (const entry of languages.yon.entries) {
      const names = entry.files.map((file) => file.name)
      expect(names.some((name) => name.startsWith('server/controllers/'))).toBe(true)
      expect(names.some((name) => name.startsWith('server/services/OrderService.'))).toBe(true)
      const controller = Found.required(entry.files.find((file) => /\/controllers\/OrdersController\./.test(file.name)))
      expect(controller.code).toMatch(/RequestRoute|request_route/)
      expect(controller.code).toMatch(/GetRoute|get_route/)
      expect(controller.code).toContain('OrderService')
      expect(controller.code).not.toMatch(/(?:class|struct|object) OrderService/)
    }
  })

  test('published test units select separate test-only configuration files in every language', async () => {
    const docs = await SiteData.docs()
    /** @type {[string, number][]} */
    const realms = [['Yon', 15], ['Tac', 14]]
    for (const [realm, count] of realms) {
      const section = Found.required(docs.topics.annotations.sections.find((entry) => entry.heading === `Test units: ${realm}`))
      const configurations = section.files.filter((file) => /\/tests\/CatalogTestSetup\./.test(file.name))
      expect(configurations).toHaveLength(count)
      for (const configuration of configurations) {
        const extension = configuration.name.split('.').pop()
        const unit = Found.required(section.files.find((file) => file.name.endsWith(`/CatalogServiceTest.${extension}`)))
        expect(configuration.code).toContain('TestSetup')
        expect(configuration.code).not.toMatch(/[@[]Test\(|#\[Test\(|@test\(/)
        expect(unit.code).toMatch(/Test\((typeof[(<])?CatalogTestSetup/)
        expect(unit.code).not.toMatch(/(?:class|struct) CatalogTestSetup/)
      }
    }
  })

  test('every service language shows the same class, spelled its own way', async () => {
    const languages = await SiteData.languages()
    const companions = languages.tac.files.filter((file) => /\.(js|ts|py|dart|rs|swift|kt|java|cs|fs|go|rb|lua)$/.test(file.name))
    expect(companions.length).toBe(13)
    for (const entry of companions) {
      const code = entry.code
      expect(code).toMatch(/\b(class|struct|type)\b/)
      for (const member of ['count', 'runtime', 'doubled']) {
        const spellings = [member, member[0].toUpperCase() + member.slice(1)]
        expect(spellings.some((name) => code.includes(name))).toBe(true)
      }
      expect(code).not.toContain('TacMember')
      expect(code).not.toContain('TacField')
    }
  })

  test('every service language marks the class as a service, spelled its own way', async () => {
    const languages = await SiteData.languages()
    /** @type {Record<string, string>} */
    const markers = { js: '@Service', ts: '@Service', py: '@service', dart: '@Service', swift: '@Service',
                      kt: '@Service', java: '@Service', cs: '[Service]', fs: '[<Service>]', rs: '#[Service]', go: '// @Service', rb: '#[Service]',
                      lua: '-- @Service', hpp: 'class [[Service]] TallyService {' }
    const services = languages.tac.files.filter((file) => file.name.startsWith('client/services/'))
    expect(services.length).toBe(14)
    for (const service of services) {
      const extension = Found.required(service.name.split('.').pop())
      expect(service.code.split('\n')).toContain(markers[extension])
    }
  })

  test('the published Python example selects the pinned offline profile', async () => {
    const languages = await SiteData.languages()
    const published = JSON.parse(Found.required(languages.tac.files.find(file => file.name === 'client/manifest.json')).code)
    expect(published.tac.python).toEqual({ runtime: 'pyodide-314.0.7', profile: 'offline' })
  })

  test('the published data is what the browser will fetch', async () => {
    const published = JSON.parse(await read('dist/web/shared/data/languages.json'))
    const authored = await SiteData.languages()
    expect(published).toEqual(authored)
  })
})

describe('discovery and metadata', () => {
  test('every page declares canonical and social metadata', async () => {
    for (const [route, document] of [['/', 'index.html'], ['/docs', 'docs/index.html'],
                                     ['/docs/features', 'docs/features/index.html']]) {
      const html = await read(`dist/web/${document}`)
      expect(html).toContain('rel="canonical"')
      expect(html).toContain('property="og:title"')
      expect(html).toContain('name="twitter:card"')
      expect(html).toContain('lang="en"')
      expect(html).toContain(`href="https://tachyon.del.ma${route === '/' ? '/' : route}"`)
    }
    const layout = await read('client/components/site/layout/tac.html')
    expect(layout).toContain('href="#main"')
    expect(layout).toContain('<site-header />')
    expect(layout).toContain('<site-footer />')
    for (const view of ['client/pages/tac.html', 'client/pages/docs/tac.html',
                        'client/pages/docs/features/tac.html']) {
      const authored = await read(view)
      expect(authored).toContain('<site-layout>')
      expect(authored).not.toContain('<site-header')
      expect(authored).not.toContain('<site-footer')
      expect(authored).not.toContain('<head>')
      expect(authored).not.toContain('<body>')
    }
    for (const document of ['index.html', 'docs/index.html', 'docs/features/index.html']) {
      const html = await read(`dist/web/${document}`)
      expect(html).toContain('#main')
      expect(html).toContain('site-header')
      expect(html).toContain('site-footer')
    }
  })

  test('the build emits a sitemap covering every guide, and robots links it', async () => {
    const docs = await SiteData.docs()
    const sitemap = await read('dist/web/sitemap.xml')
    for (const slug of docs.order) expect(sitemap).toContain(`/docs/${slug}<`)
    expect(sitemap).not.toContain('_topic')
    const robots = await read('dist/web/robots.txt')
    expect(robots).toContain('Sitemap:')
  })
})

describe('feature catalogue', () => {
  test('covers every group and carries a snippet for each feature', async () => {
    const { features } = await SiteData.features()
    const groups = new Set(features.map((feature) => feature.group))
    for (const group of ['Routing', 'Tac views', 'Browser storage', 'Polyglot',
                         'Yon server', 'Native', 'Tooling']) {
      expect(groups).toContain(group)
    }
    for (const feature of features) {
      expect(feature.files.length).toBeGreaterThan(0)
      for (const file of feature.files) {
        expect(file.code.trim().length).toBeGreaterThan(10)
        expect(file.name).not.toMatch(/^server\/routes\//)
      }
    }
    const ids = features.map((feature) => feature.id)
    expect(ids.length).toBe(new Set(ids).size)
  })

  test('documents the framework surface the ledger claims', async () => {
    const { features } = await SiteData.features()
    const text = JSON.stringify(features)
    for (const probe of ['_id', '<logic :if', '<loop :for', '<switch', '<slot',
                         'hydrate=', 'on:click', '@LocalState', '\\"policy\\"', 'tachyon-sw',
                         'Service.rs', 'Service.swift', 'Service.kt', 'Service.cs', 'Service.go',
                         '@Service\\n', 'Lifetime({', 'client/services/',
                         'Controller.js', 'Controller.py',
                         'Controller.php', 'Controller.kt', 'Controller.java', 'middleware', '@StreamRequest', 'async *stream',
                         'companion.invoke', 'start_url', 'ty doctor',
                          'ty start', 'TY1307', 'static class fields',
                         'ty migrate check', 'tac.sitemap', 'hotState', 'GetRoute']) {
      expect(text).toContain(probe)
    }
  })

  test('documents Yon as a REST boundary, never a renderer', async () => {
    const docs = await SiteData.docs()
    const { features } = await SiteData.features()
    const yon = JSON.stringify(docs.topics.models)
    const handlers = JSON.stringify(features.find((feature) => feature.id === 'handlers'))
    for (const text of [yon, handlers]) {
      expect(text).toContain('text/html')
      expect(text.toLowerCase()).toContain('yon')
      expect(text.toLowerCase()).toContain('render')
    }
    expect(yon).toContain('yon.html')
    expect(yon).toContain('TY1008')
    expect(handlers).toContain('{not-a-template}')
  })

  test('the removed releases page leaves nothing behind', async () => {
    const header = await read('client/components/site/header/tac.html')
    expect(header).not.toContain('/releases')
  })
})

describe('house style', () => {
  test('every page section uses the shared eyebrow, title and lede', async () => {
    const sheets = await Promise.all(['home/hero', 'home/pillars', 'home/quickstart'].map(owner => read(`client/components/${owner}/tac.css`)))
    for (const rule of ['.eyebrow', '.section-title', '.section-lede', '.panel', '.command', '.stat-value']) {
      expect(sheets.join('\n')).toContain(rule)
    }
    expect(await read('client/shared/styles/site.css')).not.toContain('.section-title')

    for (const component of ['home/hero', 'home/pillars', 'home/quickstart', 'home/faq']) {
      const html = await read(`client/components/${component}/tac.html`)
      expect(html).toContain('class="eyebrow"')
    }
  })

  test('a first visit lands on the dark theme, and a saved theme stays', async () => {
    const source = await read('client/shared/scripts/ThemeDefault.js')
    const root = new Map()
    const page = { documentElement: { setAttribute: /** @param {string} name @param {string} value */ (name, value) => root.set(name, value) } }
    /** @param {string | null} saved */
    const run = (saved) => {
      const values = new Map(saved === null ? [] : [['w-theme', saved]])
      /** @type {Pick<Storage, 'getItem' | 'setItem'>} */
      const storage = { getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }
      root.clear()
      new Function('localStorage', 'document', source)(storage, page)
      expect(root.get('w-theme')).toBe(values.get('w-theme'))
      return values.get('w-theme')
    }
    expect(run(null)).toBe('dark')
    expect(run('light')).toBe('light')
    expect(run('auto')).toBe('auto')
    const blocked = { getItem: () => { throw new Error('denied') }, setItem: () => { throw new Error('denied') } }
    expect(() => new Function('localStorage', 'document', source)(blocked, page)).toThrow('denied')

    const manifest = await readJson('client/manifest.json')
    expect(manifest.tac.blockingScripts).toEqual(['/shared/scripts/ThemeDefault.js'])
    expect(manifest.tac.scripts).not.toContain('/shared/scripts/ThemeDefault.js')
    const css = await read('client/shared/styles/site.css')
    expect(css).toContain('--page-background: var(--tachyon-night);')
    expect(css).toContain('background: var(--page-background);')
    expect(css).toContain(':root[w-theme="dark"],\n:root[w-theme="light"],\n:root[w-theme="auto"] {\n--page-background: var(--w-surface);')
  })

  test('no page restates a number the data already carries', async () => {
    const components = await readdir(fileURLToPath(new URL('../client/components/site', import.meta.url)))
    expect(components).not.toContain('stats')
    const home = await read('client/pages/tac.html')
    expect(home).not.toContain('site-stats')
  })
})

describe('responsive system', () => {
  test('layout is driven by space, not by a breakpoint list', async () => {
    const sheets = await Promise.all(['site/footer', 'docs/catalog', 'docs/layout'].map(owner => read(`client/components/${owner}/tac.css`)))
    const css = sheets.join('\n')
    expect(css).toContain('repeat(auto-fit, minmax(min(100%, var(--col')
    expect(css).toContain('grid-auto-rows: 1fr')
    expect(css).toContain('container-type: inline-size')
    expect(css).toContain('@container (min-width: 52rem)')
    expect(css).toContain('@container (min-width: 68rem)')
  })

  test('no template reintroduces a breakpoint grid', async () => {
    for (const template of ['site/footer', 'home/quickstart']) {
      const html = await read(`client/components/${template}/tac.html`)
      expect(html).not.toMatch(/\b(sm|md|lg|xl):grid-cols-/)
      expect(html).toContain('grid-auto')
    }
    for (const stylesheet of ['client/components/site/footer/tac.css',
                              'client/components/home/pillars/tac.css']) {
      const css = await read(stylesheet)
      expect(css).toContain('auto-fit')
    }
    expect(await read('client/components/docs/catalog/tac.css')).toContain('.catalogue__card')
  })

  test('the type scale interpolates rather than stepping', async () => {
    const css = await read('client/shared/styles/site.css')
    for (const step of ['--step-xs', '--step-sm', '--step-lg', '--step-2xl', '--gutter', '--section-space']) {
      expect(css).toMatch(new RegExp(`${step}: clamp\\(`))
    }
    expect(css).toMatch(/--step-2xl: clamp\([^)]*vw/)
  })

  test('the type scale matches the average of other framework websites', async () => {
    const css = await read('client/shared/styles/site.css')
    expect(css).toContain('--step-nav: .9375rem;')
    expect(css).toContain('--step-3xl: clamp(1.75rem, 1.2rem + 1.6vw, 2.5rem);')
    expect(css).toContain('--step-xl: clamp(1.25rem, 1rem + .8vw, 1.5rem);')
    expect(css).toContain('--step-hero: clamp(2.5rem, 7.5vw, 3.5rem);')
    expect(css).toContain('--inline-code-size: max(.75rem, .85em);')
    expect(css).toContain('--body-line-height: 1.7;')
    expect(css).toContain('--text-measure: 68ch;')
    expect(css).not.toMatch(/font-size: clamp\(/)
    expect(await read('client/components/docs/sidebar/tac.css')).toContain('font-size: var(--step-nav);')
    expect(await read('client/components/site/header/tac.css')).toContain('font-size: var(--step-nav);')
    expect(await read('client/components/home/pillars/tac.css')).toContain('font-size: var(--step-base);')
  })

  test('the site answers to more than viewport width', async () => {
    const css = await read('client/shared/styles/site.css')
    expect(await read('client/components/site/header/tac.css')).toContain('@media (pointer: coarse)')
    expect(css).toContain('@media (prefers-contrast: more)')
    expect(css).toContain('@media (prefers-reduced-motion: reduce)')

    const shell = await read('client/components/docs/layout/tac.css')
    expect(shell).toContain('.docs-frame')
    expect(shell).toContain('@container (min-width: 52rem)')
    expect(shell).toContain('@container (min-width: 68rem)')
    expect(shell).not.toMatch(/@media[^{]*max-width[^{]*\{\s*\.docs-shell/)
  })
})

describe('documentation structure', () => {
  test('every footer documentation link names a published page', async () => {
    const footer = await SiteData.footer()
    const links = footer.flatMap((column) => column.links.map((link) => link.href))
    expect(links.length).toBeGreaterThan(5)
    const docs = await SiteData.docs()
    const published = ['/docs', '/docs/features', ...docs.order.map((slug) => `/docs/${slug}`)]
    for (const href of links) {
      if (href.startsWith('/docs')) expect(published).toContain(href)
    }
  })

  test('every feature is a page of its own', async () => {
    const routes = await readdir(fileURLToPath(new URL('../client/pages/docs/features', import.meta.url)))
    expect(routes).toContain('tac.html')
    expect(routes).toContain('_id')

    const catalogue = await SiteData.features()
    for (const feature of catalogue.features) {
      expect(feature.id).toMatch(/^[a-z0-9]+(-[a-z0-9]+)*$/)
    }
    const ids = catalogue.features.map((feature) => feature.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  test('the sidebar reaches every page, and the pager passes through all of them', async () => {
    const catalogue = await SiteData.features()
    const groups = [...new Set(catalogue.features.map((feature) => feature.group))]
    for (const group of groups) expect(catalogue.groups).toContain(group)
    const route = 'dist/web/docs/_topic/index.html'
    for (const [component, name] of [['docs-sidebar', 'sidebar'], ['docs-pager', 'pager']]) {
      expect(Found.required(await Built.service(route, component, name)).required).toEqual(expect.arrayContaining(['documents', 'features']))
    }
  })

  test('every page offers the search', async () => {
    for (const route of Built.routes) {
      expect(await Built.service(route, 'site-search', 'search'), route).toBeDefined()
    }
  })

  test('the sitemap lists the feature pages', async () => {
    const { tac } = JSON.parse(await read('client/manifest.json'))
    expect(tac.sitemap['/docs/features/_id']).toEqual({ data: '/shared/data/features.json', values: '/features', field: 'id' })
    expect(tac.sitemap['/docs/_topic']).toEqual({ data: '/shared/data/docs.json', values: '/order' })
    expect(tac.metadata.routes['/docs/features']).toBeDefined()
    expect(tac.metadata.routes['/docs/features/_id'].noindex).toBe(true)
    expect(existsSync(new URL('../tac.config.js', import.meta.url))).toBe(false)
    expect(existsSync(new URL('../tachyon.json', import.meta.url))).toBe(false)
  })

  test('a diagnostic the site quotes still says what the code emits', async () => {
    /** @param {string} text */
    const flatten = (text) =>
      text
        .replace(/\\\s*\n\s*/g, '')
        .replace(/\\"/g, '"')
        .replace(/\s+/g, ' ')
    const coreSource = fileURLToPath(new URL('../../crates/tachyon-core/src', import.meta.url))
    const rustFiles = (await listFiles(coreSource)).filter((file) => file.name.endsWith('.rs'))
    const corpus = flatten(
      (await Promise.all(rustFiles.map((file) => readFile(file.path, 'utf8')))).join('\n'),
    )

    const catalogue = await SiteData.features()
    const quoted = []
    for (const feature of catalogue.features.filter((one) => one.id === 'diagnostics')) {
      for (const file of feature.files) {
        for (const block of file.code.split(/\n\s*\n/)) {
          const code = block.match(/error\[(TY\d{4})\]/)?.[1]
          const help = block.split('help:')[1]
          if (code && help) quoted.push({ code, help: flatten(help).trim() })
        }
      }
    }
    expect(quoted.length).toBeGreaterThan(2)

    let checked = 0
    for (const { code, help } of quoted) {
      for (const fragment of help.split(/'[^']*'/).map((part) => part.trim())) {
        if (fragment.length < 24) continue
        checked += 1
        expect(
          corpus.includes(fragment),
          `${code} on the site says "${fragment}", which is not what the code emits`,
        ).toBe(true)
      }
    }
    expect(checked).toBeGreaterThan(2)
  })

  test('a Tac example shows its view and its service together', async () => {
    const catalogue = await SiteData.features()
    /** @param {string} name */
    const isServiceFile = (name) => /^client\/services\/[A-Z]\w*Service\.(js|ts|swift|kt|cs|rs|go)$/.test(name)

    const tacFeatures = catalogue.features.filter((feature) =>
      feature.files.some((file) => isServiceFile(file.name)))
    expect(tacFeatures.length).toBeGreaterThan(15)

    for (const feature of tacFeatures) {
      const names = feature.files.map((file) => file.name)
      expect(names.some((name) => /^client\/components\/.*\/tac\.html$/.test(name)), feature.id).toBe(true)
      expect(names.some((name) => /^client\/services\/\w+Service\.(js|ts|dart|rs|cs|go)$/.test(name)), feature.id).toBe(true)
      for (const name of names) {
        if (name.startsWith('client/components/')) expect(name, feature.id).toMatch(/\/tac\.(html|css)$/)
        if (name.startsWith('client/pages/')) expect(name, feature.id).toMatch(/\/tac\.html$/)
      }
    }
  })

  test('an example reads behaviour, then structure, then presentation', async () => {
    const { PlatformUtils: { inReadingOrder } } = await import('../client/shared/utils/PlatformUtils.js')

    /** @param {string} extension */
    const service = (extension) => `client/services/TallyService.${extension}`
    /** @param {string[]} names */
    const ordered = (names) => inReadingOrder(Snippets.named(...names)).map((file) => file.name)
    expect(ordered(['emitted', 'tac.css', 'tac.html', service('go'), service('rs'), service('swift'), service('ts'), service('js')]))
      .toEqual([service('js'), service('ts'), service('swift'), service('rs'), service('go'), 'tac.html', 'tac.css', 'emitted'])

    const shuffled = Snippets.named('emitted', 'tac.css', 'tac.html', 'client/services/TallyService.swift')
    expect(inReadingOrder(shuffled).map((file) => file.name))
      .toEqual(['client/services/TallyService.swift', 'tac.html', 'tac.css', 'emitted'])
  })

  test('the framework layers are rendered together, not switched between', async () => {
    const html = await read('client/components/home/languages/tac.html')
    expect(html).toContain(`:hidden="showcase.layer !== 'tac'"`)
    expect(html).toContain(`:hidden="showcase.layer !== 'yon'"`)
    expect(html).not.toMatch(/<logic[^>]*>\s*<docs-snippet/)
    expect(html).not.toContain('<logic else>')
  })

  test('the view and the styles are shown once, whatever the service language', async () => {
    const { PlatformUtils: { inReadingOrder, isCompanion } } = await import('../client/shared/utils/PlatformUtils.js')
    const languages = await SiteData.languages()
    const files = languages.tac.files

    expect(files.filter((file) => file.name.endsWith('.html'))).toHaveLength(1)
    expect(files.filter((file) => file.name.endsWith('.css'))).toHaveLength(1)

    expect(isCompanion('client/components/basket/tally/tac.html')).toBe(false)
    expect(isCompanion('client/components/basket/tally/tac.css')).toBe(false)
    expect(isCompanion('client/services/TallyService.swift')).toBe(true)

    const ordered = inReadingOrder(files)
    expect(ordered.filter(file => file.name === 'client/manifest.json')).toHaveLength(1)
    const order = ordered.filter(file => file.name !== 'client/manifest.json').map((file) => Found.required(file.name.split('/').pop()))
    expect(order.at(-2)).toBe('tac.html')
    expect(order.at(-1)).toBe('tac.css')
    expect(order.slice(0, -2).every(name => isCompanion(name))).toBe(true)
  })

  test('a platform strip only appears where the example actually differs', async () => {
    const { PlatformUtils: { spansPlatforms, filesFor } } =
      await import('../client/shared/utils/PlatformUtils.js')

    /** @param {string} extension */
    const service = (extension) => `client/services/TallyService.${extension}`
    /** @param {SnippetFile[]} files */
    const names = (files) => files.map((file) => file.name)
    expect(spansPlatforms(Snippets.named(service('js')))).toBe(false)
    expect(spansPlatforms(Snippets.named(service('js'), service('go')))).toBe(false)
    expect(spansPlatforms(Snippets.named(service('js'), service('swift')))).toBe(false)
    expect(spansPlatforms(Snippets.named(service('js'), service('py')))).toBe(true)

    const everyPlatform = ['web', 'macos', 'ios', 'android', 'windows', 'linux']
    /** @param {string} name */
    const reachOf = (name) => everyPlatform.filter((platform) => filesFor(Snippets.named(name), platform).length > 0)
    expect(reachOf(service('dart'))).toEqual(everyPlatform)
    expect(reachOf(service('py'))).toEqual(['web'])
    expect(reachOf(service('go'))).toEqual(everyPlatform)
    expect(reachOf(service('rb'))).toEqual(everyPlatform)
    expect(reachOf(service('lua'))).toEqual(everyPlatform)
    expect(reachOf(service('rs'))).toEqual(everyPlatform)
    expect(reachOf(service('cs'))).toEqual(everyPlatform)
    expect(reachOf(service('swift'))).toEqual(everyPlatform)
    expect(reachOf(service('kt'))).toEqual(everyPlatform)
    expect(reachOf('client/components/basket/tally/tac.html')).toContain('android')

    const mixed = Snippets.named(service('py'), service('swift'), service('kt'), service('js'))
    expect(names(filesFor(mixed, 'android'))).toEqual([service('swift'), service('kt'), service('js')])
    expect(names(filesFor(mixed, 'web'))).toEqual([service('py'), service('swift'), service('kt'), service('js')])

    const view = 'client/components/basket/tally/tac.html'
    const webAssembly = Snippets.named(service('rs'), service('go'), service('cs'), view)
    expect(names(filesFor(webAssembly, 'web'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor(webAssembly, 'windows'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor(webAssembly, 'linux'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor(webAssembly, 'ios'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor(Snippets.named(service('cs')), 'android'))).toEqual([service('cs')])
    expect(filesFor(Snippets.named(service('py')), 'ios')).toEqual([])
  })

  test('each home page platform tab shows every service language that can run there', async () => {
    const { PlatformUtils: { filesFor, inReadingOrder, isCompanion, languageLabel } } =
      await import('../client/shared/utils/PlatformUtils.js')
    const languages = await SiteData.languages()
    /** @param {string} platform */
    const shownOn = (platform) => inReadingOrder(filesFor(languages.tac.files, platform))
      .filter((file) => isCompanion(file.name))
      .map((file) => languageLabel(file.name))

    const browser = ['JavaScript', 'TypeScript', 'Dart', 'Java']
    const everywhere = ['Swift', 'Kotlin', 'C#', 'F#', 'Rust', 'Go', 'Ruby', 'Lua', 'C++']
    expect(shownOn('web')).toEqual(['JavaScript', 'TypeScript', 'Dart', 'Python', 'Java', ...everywhere])
    expect(shownOn('macos')).toEqual([...browser, ...everywhere])
    expect(shownOn('ios')).toEqual([...browser, ...everywhere])
    expect(shownOn('android')).toEqual([...browser, ...everywhere])
    expect(shownOn('windows')).toEqual([...browser, ...everywhere])
    expect(shownOn('linux')).toEqual([...browser, ...everywhere])
  })

  test('the navigation is a drawer where the shell has no column for it', async () => {
    const css = await read('client/components/docs/sidebar/tac.css')
    expect(css).toContain('@container (max-width: 51.999rem)')
    expect(css).toContain('visibility: hidden')

    expect(css).toContain('.docs-nav__section[open] > .docs-nav__list')
    expect(css).toContain('.docs-nav__section:not([open]) > .docs-nav__list')

    const html = await read('client/components/docs/sidebar/tac.html')
    expect(html).toContain('aria-expanded')
    expect(html).toContain('aria-controls="docs-nav-panel"')
    expect(html).toContain('on:click="drawer.toggle()"')
  })

  test('the docs shell dissolves its wrappers where the rule can win', async () => {
    const layout = await read('client/components/docs/layout/tac.css')
    expect(layout).toContain('display: contents')
    expect(layout).toContain('data-tachyon-component')
    const site = await read('client/shared/styles/site.css')
    expect(site).not.toMatch(/^\.docs-page \{\n\s*display: contents/m)
  })
})
