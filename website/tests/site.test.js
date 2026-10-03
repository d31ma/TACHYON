// @ts-nocheck

import { SiteArtifacts } from '../../scripts/site-artifacts.mjs'
import { beforeAll, describe, expect, test } from 'bun:test'
import { existsSync } from 'node:fs'
import { readdir, readFile } from 'node:fs/promises'
import { fileURLToPath } from 'node:url'
import { ensureBundle } from './helpers/ensure-bundle.js'

class Patterns {
  static needs = /[@#[<]+Needs(\(\))?[\s\]>]|Annotated\[[^\]]*\bneeds\b/
}

function read(relative) {
  if (relative.startsWith('dist/web/')) {
    const project = fileURLToPath(new URL('..', import.meta.url))
    return readFile(`${SiteArtifacts.web(project)}/${relative.slice(9)}`, 'utf8')
  }
  return readFile(new URL(relative, new URL('..', import.meta.url)), 'utf8')
}

async function readJson(relative) {
  return JSON.parse(await read(relative))
}

async function listFiles(directory) {
  const files = []
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const child = `${directory}/${entry.name}`
    if (entry.isDirectory()) files.push(...await listFiles(child))
    else files.push({ path: child, name: entry.name })
  }
  return files
}

function instanceNameOf(className) {
  const stem = className.slice(0, -'Service'.length)
  return stem[0].toLowerCase() + stem.slice(1)
}

function expectServiceShape(source, className, extension) {
  if (extension === 'lua') {
    expect(source).toMatch(/^-- @Service$/m)
    expect(source).toContain(`---@class ${className}`)
    expect(source).toContain(`${className} = `)
    return
  }
  if (extension === 'py') {
    expect(source).toMatch(/^@service$/m)
    expect(source).toContain(`class ${className}:`)
    return
  }
  if (extension === 'java') {
    expect(source).toMatch(/^@Service$/m)
    expect(source).toContain(`class ${className} {`)
    return
  }
  if (extension === 'go') {
    expect(source).toMatch(/^\/\/ @Service$/m)
    expect(source).toContain(`type ${className} struct`)
    return
  }
  if (extension === 'rs') {
    expect(source).toMatch(/^#\[Service\]$/m)
    expect(source).toContain(`struct ${className} {`)
    expect(source).toContain(`impl ${className} {`)
    return
  }
  if (extension === 'rb') {
    expect(source).toMatch(/^#\[Service\]$/m)
    expect(source).toContain(`class ${className}\n`)
    return
  }
  if (extension === 'cs') {
    expect(source).toMatch(/^\[Service\]$/m)
    expect(source).toContain(`public class ${className}`)
    return
  }
  if (extension === 'fs') {
    expect(source).toMatch(/^\[<Service>\]$/m)
    expect(source).toContain(`type ${className}() =`)
    return
  }
  expect(source).toMatch(/^@Service$/m)
  if (extension === 'dart' || extension === 'kt') {
    expect(source).toContain(`class ${className} `)
    return
  }
  if (extension === 'swift') {
    expect(source).toContain(`final class ${className} `)
    return
  }
  expect(source).toContain(`export default class ${className} `)
  expect(source).not.toContain('tachyon:behaviors')
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
      const html = await read(route)
      const view = JSON.parse(html.match(/<script id="tachyon-view"[^>]*>([\s\S]*?)<\/script>/)[1])
      const pending = [view]
      while (pending.length > 0) {
        const node = pending.pop()
        if (Array.isArray(node)) pending.push(...node)
        else if (node && typeof node === 'object') {
          pending.push(...Object.values(node))
          if (node.k !== 'component') continue
          const receivers = node.services.filter(service => Array.isArray(service.required))
          if (receivers.length === 0) continue
          const required = new Set(receivers.flatMap(service => service.required))
          for (const property of node.properties ?? []) {
            if (!required.has(property.name)) lost.push(`${route}: <${node.name} :${property.name}>`)
          }
        }
      }
    }
    expect(lost).toEqual([])
  })

  test('interactive services inject owner-scoped render resources', async () => {
    for (const file of ['ClipboardService.ts', 'SearchService.js']) {
      const source = await read(`client/services/${file}`)
      expect(source).toMatch(/this\.taskScheduler\??\.render\(\)/)
      expect(source).toMatch(/@Needs\n\s*taskScheduler\b/)
      expect(source).not.toContain('@Resource')
      expect(source).not.toContain('this.tac')
      expect(source).not.toContain('globalThis.__tachyonTac')
      expect(source).not.toContain('__tachyonOnMount')
    }
  })

  test('live site services use OnStart instead of implicit mount hooks', async () => {
    const manifest = JSON.parse(await read('client/manifest.json'))
    expect(manifest.tac.metadata.routes['/']).toBeDefined()
    for (const sourcePath of [
      'client/services/FeatureService.kt',
      'client/services/TopicService.ts',
      'client/services/SearchService.js',
    ]) {
      const source = await read(sourcePath)
      expect(source).toContain('@OnStart')
      expect(source).not.toContain('mount(')
      expect(source).not.toContain('__tachyonOnMount')
    }
  })

  test('the Kotlin feature page receives genuine catalogue props and an injected title provider', async () => {
    const source = await read('client/services/FeatureService.kt')
    expect(source).toMatch(/^@NeedsAll$/m)
    expect(source).toMatch(/@Needs\n\s*private val documentTitle: DocumentTitle/)
    expect(source).toContain('lateinit var route: String')
    expect(source).toContain('lateinit var catalogue: FeatureCatalogue')
    expect(source).not.toContain('/shared/scripts/navigation.js')
    expect(source).not.toContain('document.title')
    expect(await read('client/pages/docs/features/_id/tac.html')).toContain(':route="page.path" :catalogue="data.features"')
    const manifest = JSON.parse(await read('client/manifest.json'))
    expect(manifest.tac.browserApis).not.toContain('documentTitle')
    const testSource = await read('client/tests/FeatureServiceTest.kt')
    expect(testSource).toContain('@Fake')
    expect(testSource).toContain('construct(FeatureService::class)')
    expect(testSource).toContain('testEntryIsTheFeatureTheUrlNames')
    expect(testSource).toContain('testAnUnknownFeatureIsNotFoundRatherThanBroken')
    expect(testSource).toContain('testInitializeTitlesTheDocumentAfterTheFeature')
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
    const instances = new Set()
    for (const file of services) {
      const [, className, extension] = file.match(/^([A-Z]\w*Service)\.(js|ts|py|dart|rs|cs|fs|go|rb|kt|swift|lua|java)$/) ?? []
      expect(className, file).toBeDefined()
      const source = await read(`client/services/${file}`)
      expectServiceShape(source, className, extension)
      instances.add(instanceNameOf(className))
    }
    const named = new Set()
    for (const view of views) {
      const html = await readFile(view, 'utf8')
      for (const [, instance] of html.matchAll(/\b([a-z][A-Za-z]*)\.[A-Za-z$]/g)) {
        if (instances.has(instance)) named.add(instance)
      }
    }
    expect([...instances].sort()).toEqual([...named].sort())
    for (const prototype of ['SnippetService', 'TocService', 'PagerService', 'PagelinksService']) {
      const file = services.find((name) => name.startsWith(`${prototype}.`))
      const source = await read(`client/services/${file}`)
      expect(source).toMatch(/^(@|#\[|\[)NeedsAll\]?$/m)
      expect(source).not.toContain('Lifetime(')
    }
    expect(await read('client/services/ClipboardService.ts')).toContain('@Lifetime({"value":"prototype"})')
  })
})

describe('language coverage', () => {
  test('application events cover every owned language without retired runtime calls', async () => {
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const topic = docs.topics['application-events']
    const browser = topic.sections.find(section => section.heading.startsWith('Browser services'))
    const compiled = topic.sections.find(section => section.files?.some(file => file.name === 'client/services/CatalogService.swift'))
    const views = /^client\/(models|pages|components)\//
    const services = (section) => section.files.filter(file => !views.test(file.name))
    for (const file of [...services(browser), ...services(compiled)]) expect(file.name).toMatch(/^client\/services\/CatalogService\.[a-z]+$/)
    const yon = topic.sections.find(section => section.heading.startsWith('Yon:'))
    expect(services(browser).map(file => file.name.split('.').pop()).sort()).toEqual(['js', 'ts'])
    expect(services(compiled).map(file => file.name.split('.').pop()).sort()).toEqual(['cs', 'dart', 'fs', 'go', 'java', 'kt', 'rb', 'rs', 'swift'])
    for (const file of compiled.files.filter(file => file.name.startsWith('client/models/'))) expect(file.name).toMatch(/^client\/models\/CatalogCount\.[a-z]+$/)
    expect(yon.files.filter(file => /\/controllers\//.test(file.name)).map(file => file.name.split('.').pop()).sort())
      .toEqual(['cs', 'dart', 'fs', 'go', 'groovy', 'java', 'js', 'kt', 'php', 'py', 'rb', 'rs', 'ts'])
    for (const section of topic.sections) for (const file of section.files ?? []) {
      expect(file.code).not.toContain('this.tac')
      expect(file.code).not.toMatch(/tac_publish|tacPublish|Tac\.Publish|\bmount\(/)
    }
  })

  test('strict role layouts have complete annotation-discovered examples in every supported language', async () => {
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const sections = docs.topics['project-folders'].sections
    for (const [framework, extensions, entryPattern, modulePattern] of [
      ['yon', ['cs', 'dart', 'fs', 'go', 'groovy', 'java', 'js', 'kt', 'lua', 'php', 'py', 'rb', 'rs', 'ts'], /^server\/controllers\/MoneyController\.[a-z]+$/, /^server\/services\/MoneyService\./],
      ['tac', ['cs', 'dart', 'fs', 'go', 'java', 'js', 'kt', 'rb', 'rs', 'swift', 'ts'], /^client\/services\/MoneyService\.[a-z]+$/, /^client\/entities\/MoneyEntity\./],
    ]) {
      const section = sections.find(entry => entry.files?.some(file => entryPattern.test(file.name)))
      expect(section.axis).toBe('language')
      expect(section.files.some(file => file.name.endsWith('.modules.json'))).toBe(false)
      const entries = section.files.filter(file => entryPattern.test(file.name))
      expect(entries.map(file => file.name.split('.').pop()).sort()).toEqual(extensions)
      if (framework === 'tac') {
        expect(section.files.some(file => file.name === 'client/pages/tac.html' && file.code.includes('<money-panel'))).toBe(true)
        expect(section.files.find(file => file.name === 'client/components/money/panel/tac.html').code).toContain('{money.amount}')
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
          const controller = section.files.find(file => file.name.endsWith(`/MoneyController.${extension}`)).code
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
    const docs = await readJson('client/shared/data/docs.json')
    const section = docs.topics.annotations.sections.find(section => section.heading === 'Configuration binding: Tac')
    const manifest = JSON.parse(section.files.find(file => file.name === 'client/manifest.json').code)
    expect(manifest.tac.catalog).toEqual({ title: 'Catalog', page: { size: 20 }, cache: { enabled: true } })
    const configurations = section.files.filter(file => file.name.startsWith('client/setups/'))
    expect(configurations.map(file => file.name.split('.').pop()).sort()).toEqual(['cs', 'dart', 'fs', 'go', 'java', 'js', 'kt', 'rb', 'rs', 'swift', 'ts'])
    for (const file of configurations) {
      const policy = JSON.parse(file.code.match(/Settings\((\{[^\n]*\})\)/)[1])
      expect(policy.prefix).toBe('TAC_CATALOG_')
      expect(policy.fields).toBeUndefined()
      const fields = file.name.endsWith('.rb') ? ['page_size', 'cache_enabled'] : ['pageSize', 'cacheEnabled']
      for (const field of [...fields, 'title', 'Min(1)', 'Max(100)']) expect(file.code).toContain(field)
    }
    const go = section.files.find(file => file.name === 'client/services/SettingsService.go').code
    expect(go).toContain('BehaviorsProperties[CatalogSetup](published)')
    expect(go).not.toContain('catalogProperties')
  })

  test('application configuration offers independently runnable examples in every Yon language', async () => {
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const section = docs.topics['application-configuration'].sections
      .find((entry) => entry.heading === 'Access from every Yon language')
    expect(section).toBeDefined()
    expect(section.axis).toBe('language')
    const handlers = section.files.filter((file) => /^server\/controllers\//.test(file.name))
    expect(handlers.map((file) => file.name.split('.').pop()).sort())
      .toEqual(['cs', 'dart', 'go', 'groovy', 'java', 'js', 'kt', 'php', 'py', 'rs', 'ts'])
    for (const handler of handlers) {
      expect(handler.name).toMatch(/^server\/controllers\/ConfigurationController\./)
      expect(handler.code).toMatch(/RequestRoute|request_route/)
      const extension = handler.name.split('.').pop()
      const controller = section.files.find((file) => file.name.endsWith(`/ConfigurationController.${extension}`))
      const settings = section.files.find((file) => file.name === `server/helpers/CatalogSettings.${extension}`)
      const endpoint = section.files.find((file) => file.name === `server/helpers/Endpoint.${extension}`)
      expect(settings.code).toMatch(/[@[]Settings\(|@settings\(/)
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
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const repositories = docs.topics['server-data'].sections.find(section => section.heading === 'DataSource in a service, in every Yon language')
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
    const clients = docs.topics['calling-the-server'].sections.find(section => section.heading === 'Client classes that call the server')
    expect(clients.body).toContain('HTTPClient')
    expect(clients.body).toContain('explicitly configured')
    for (const extension of ['js', 'ts']) {
      const provider = clients.files.find(file => file.name.endsWith(`/remotes/CatalogClient.${extension}`))
      expect(provider.code).toMatch(/^@HTTPClient\b/m)
      expect(provider.code).toContain('@GetRequest(')
      const service = clients.files.find(file => file.name === `client/services/CatalogService.${extension}`)
      expect(service.code).toMatch(Patterns.needs)
      expect(service.code).toContain('export default class CatalogService')
      expect(service.code).not.toMatch(/\bconstructor\s*\(|@Needs\(|\bfetch\s*\(/)
    }
    expect(clients.files.find(file => file.name === 'client/components/catalog/page/tac.html').code).toContain('catalog.load()')
    expect(clients.files.find(file => file.name === 'client/pages/tac.html').code).toContain('<catalog-page')
    expect(JSON.stringify(docs)).not.toMatch(/\.modules?\.json/)
  })

  test('first-app Dart tests cumulatively construct the HTTP-enabled service with typed fakes', async () => {
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const sections = docs.topics['first-app'].sections
    const step4 = sections.find(section => section.heading.startsWith('Step 4:'))
    expect(step4.axis).toBe('language')
    const source = name => step4.files.find(file => file.name === name).code
    for (const extension of ['js', 'ts', 'dart']) {
      expect(source(`client/remotes/GreetingClient.${extension}`)).toContain('@GetRequest(')
      expect(source(`client/services/CounterService.${extension}`)).toContain('@Needs')
    }
    expect(source('client/remotes/GreetingClient.dart')).toContain('Future<Greeting> getGreeting();')
    expect(source('client/services/CounterService.dart')).toContain('Future<void> askServer() async')
    expect(source('client/services/CounterService.dart')).toContain('await greetingClient.getGreeting()')
    expect(source('client/services/CounterService.dart')).not.toContain('GreetingClient(')
    expect(source('client/services/CounterService.dart')).not.toContain('fetch(')
    expect(source('client/setups/GreetingSetup.dart')).toContain('return BehaviorAdapters();')
    expect(step4.body).toContain('Credentials are omitted')
    expect(step4.body).toContain('same-origin only')
    const step5 = sections.find(section => section.heading.startsWith('Step 5:'))
    const unit = step5.files.find(file => file.name === 'client/tests/CounterServiceTest.dart').code
    expect(unit).toContain('@Fake')
    expect(unit).toContain('GreetingClient? greetingClient;')
    expect(unit).toContain('construct<CounterService>(CounterService)')
    expect(unit).toContain('await counter.askServer();')
    expect(unit).toContain('identical(firstClient, counter.greetingClient)')
    expect(unit).not.toContain('CounterService();')
    const setup = step5.files.find(file => file.name === 'client/tests/HelloTestSetup.dart').code
    expect(setup).toContain("'greeting': BehaviorHttpAdapter(")
    expect(setup).toContain("'message': 'Hi'")
    expect(step5.body).toContain('without a network request')
    expect(step5.files.some(file => file.name === 'server/tests/GreetingControllerTest.dart')).toBe(true)
  })

  test('application HTTP recipes use an HTTPClient, and cached reads use Cached on the client', async () => {
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    for (const section of [docs.topics.cookbook.sections[1], docs.topics['application-configuration'].sections.find(section => section.heading.startsWith('Public Tac output'))]) {
      expect(section.axis).toBe('language')
      expect(section.files.some(file => /\/remotes\/\w+Client\.js$/.test(file.name))).toBe(true)
      expect(section.files.some(file => /\/remotes\/\w+Client\.ts$/.test(file.name))).toBe(true)
      expect(section.files.map(file => file.code).join('\n')).not.toMatch(/\bfetch\s*\(/)
    }
    const features = JSON.parse(await read('client/shared/data/features.json')).features
    for (const feature of features) {
      const code = (feature.files || []).map(file => file.code).join('\n')
      expect(code).not.toMatch(/\bfetch\s*\(/)
    }
    const worker = features.find(feature => feature.id === 'fetch-cache')
    expect(worker.summary).toContain('TY1361')
    expect(worker.summary).toContain('public read')
    for (const extension of ['js', 'ts']) {
      const client = worker.files.find(file => file.name === `client/remotes/ProductsClient.${extension}`)
      expect(client.code).toContain('@Cached')
      expect(client.code).toContain('@ClearCache({"method":"list"})')
      const service = worker.files.find(file => file.name === `client/services/CatalogService.${extension}`)
      expect(service.code).toContain('this.productsClient.list()')
      expect(service.code).toContain('this.productsClient.save(')
    }
  })

  test('application examples separate secrets, and Tac configuration lives in the web app manifest', async () => {
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const topic = docs.topics['application-configuration']
    const files = topic.sections[0].files
    const application = JSON.parse(files.find(file => file.name === 'server/application.json').code)
    expect(application.yon).toBeUndefined()
    expect(application.tac).toBeUndefined()
    expect(application.server).toBeDefined()
    expect(JSON.stringify(application)).not.toMatch(/password|signing|\$\{/)
    const manifest = JSON.parse(files.find(file => file.name === 'client/manifest.json').code)
    expect(manifest.start_url).toBe('/')
    expect(manifest.tac).toEqual({ api: { url: '/api' } })
    expect(files.find(file => file.name === '.env').code).toContain('YON_DATASOURCE_PASSWORD=')
    expect(files.find(file => file.name === '.gitignore').code).toContain('.env')
    const published = JSON.parse(await read('dist/web/manifest.webmanifest'))
    expect(published).toEqual(JSON.parse(await read('client/manifest.json')))
    expect(published.yon).toBeUndefined()
    expect(existsSync(`${SiteArtifacts.web(fileURLToPath(new URL('..', import.meta.url)))}/.tachyon/application.json`)).toBe(false)
  })

  test('the Swift configuration example keeps native Tac separate from private Yon configuration', async () => {
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const section = docs.topics['application-configuration'].sections
      .find((entry) => entry.files?.some((file) => file.name.endsWith('Service.swift')))
    expect(section).toBeDefined()
    const prose = JSON.stringify({ heading: section.heading, body: section.body })
    expect(prose).toMatch(/Swift/)
    expect(prose).toMatch(/native Tac/i)
    expect(prose).toMatch(/public/i)
    expect(prose).toMatch(/private/i)
    expect(prose).toMatch(/Yon/)
    expect(section.files.some((file) => /\/yon\./.test(file.name))).toBe(false)
    const swift = section.files.find((file) => file.name.endsWith('Service.swift'))
    expect(swift.name).toBe('client/services/SettingsService.swift')
    expect(section.files.find((file) => file.name.endsWith('/tac.html') && file.name.startsWith('client/components/')).code).toContain('{settings.')
    expect(swift.code).toContain('PublicValues')
    expect(swift.code).not.toContain('YON_CATALOG_PAGE_SIZE')
  })

  test('every language Tachyon supports has a snippet in both layers', async () => {
    const languages = JSON.parse(await read('client/shared/data/languages.json'))
    const tac = languages.tac.files.map((file) => file.name.split('.').pop())
    const yon = languages.yon.entries.map((entry) => entry.id)

    for (const extension of ['js', 'ts', 'py', 'dart', 'rs', 'swift', 'kt', 'cs', 'fs', 'go', 'rb', 'lua']) {
      expect(tac).toContain(extension)
    }
    for (const extension of ['html', 'css']) {
      expect(tac).toContain(extension)
    }
    for (const id of ['javascript', 'typescript', 'python', 'java', 'groovy', 'csharp', 'fsharp',
                      'kotlin', 'php', 'rust', 'dart', 'go', 'ruby', 'lua']) {
      expect(yon).toContain(id)
    }
    expect(yon).toHaveLength(14)
    for (const file of languages.tac.files) {
      expect(file.code.length).toBeGreaterThan(20)
      expect(file.name).toMatch(/^client\/manifest\.json$|^client\/components\/basket\/tally\/tac\.(html|css)$|^client\/services\/TallyService\.[a-z]+$/)
    }
    expect(languages.tac.files.find((file) => file.name.endsWith('/tac.html')).code).toContain('{tally.')
    for (const entry of languages.yon.entries) {
      const names = entry.files.map((file) => file.name)
      expect(names.some((name) => name.startsWith('server/controllers/'))).toBe(true)
      expect(names.some((name) => name.startsWith('server/services/OrderService.'))).toBe(true)
      const controller = entry.files.find((file) => /\/controllers\/OrdersController\./.test(file.name))
      expect(controller.code).toMatch(/RequestRoute|request_route/)
      expect(controller.code).toMatch(/GetRoute|get_route/)
      expect(controller.code).toContain('OrderService')
      expect(controller.code).not.toMatch(/(?:class|struct|object) OrderService/)
    }
  })

  test('published test units select separate test-only configuration files in every language', async () => {
    const docs = await readJson('client/shared/data/docs.json')
    for (const [realm, count] of [['Yon', 14], ['Tac', 12]]) {
      const section = docs.topics.annotations.sections.find((entry) => entry.heading === `Test units: ${realm}`)
      const configurations = section.files.filter((file) => /\/tests\/CatalogTestSetup\./.test(file.name))
      expect(configurations).toHaveLength(count)
      for (const configuration of configurations) {
        const extension = configuration.name.split('.').pop()
        const unit = section.files.find((file) => file.name.endsWith(`/CatalogServiceTest.${extension}`))
        expect(configuration.code).toContain('TestSetup')
        expect(configuration.code).not.toMatch(/[@[]Test\(|#\[Test\(|@test\(/)
        expect(unit.code).toMatch(/Test\((typeof[(<])?CatalogTestSetup/)
        expect(unit.code).not.toMatch(/(?:class|struct) CatalogTestSetup/)
      }
    }
  })

  test('every service language shows the same class, spelled its own way', async () => {
    const languages = JSON.parse(await read('client/shared/data/languages.json'))
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
    const languages = JSON.parse(await read('client/shared/data/languages.json'))
    const markers = { js: '@Service', ts: '@Service', py: '@service', dart: '@Service', swift: '@Service',
                      kt: '@Service', java: '@Service', cs: '[Service]', fs: '[<Service>]', rs: '#[Service]', go: '// @Service', rb: '#[Service]',
                      lua: '-- @Service' }
    const services = languages.tac.files.filter((file) => file.name.startsWith('client/services/'))
    expect(services.length).toBe(13)
    for (const service of services) {
      const extension = service.name.split('.').pop()
      expect(service.code.split('\n')).toContain(markers[extension])
    }
  })

  test('the published Python example selects the pinned offline profile', async () => {
    const languages = JSON.parse(await read('client/shared/data/languages.json'))
    const published = JSON.parse(languages.tac.files.find(file => file.name === 'client/manifest.json').code)
    expect(published.tac.python).toEqual({ runtime: 'pyodide-314.0.7', profile: 'offline' })
  })

  test('the published data is what the browser will fetch', async () => {
    const published = JSON.parse(await read('dist/web/shared/data/languages.json'))
    const authored = JSON.parse(await read('client/shared/data/languages.json'))
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
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const sitemap = await read('dist/web/sitemap.xml')
    for (const slug of docs.order) expect(sitemap).toContain(`/docs/${slug}<`)
    expect(sitemap).not.toContain('_topic')
    const robots = await read('dist/web/robots.txt')
    expect(robots).toContain('Sitemap:')
  })
})

describe('feature catalogue', () => {
  test('covers every group and carries a snippet for each feature', async () => {
    const { features } = JSON.parse(await read('client/shared/data/features.json'))
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
    const { features } = JSON.parse(await read('client/shared/data/features.json'))
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
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const { features } = JSON.parse(await read('client/shared/data/features.json'))
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
    const run = (saved) => {
      const values = new Map(saved === null ? [] : [['w-theme', saved]])
      const storage = { getItem: (key) => values.get(key) ?? null, setItem: (key, value) => values.set(key, value) }
      new Function('localStorage', source)(storage)
      return values.get('w-theme')
    }
    expect(run(null)).toBe('dark')
    expect(run('light')).toBe('light')
    expect(run('auto')).toBe('auto')
    const blocked = { getItem: () => { throw new Error('denied') }, setItem: () => { throw new Error('denied') } }
    expect(() => new Function('localStorage', source)(blocked)).not.toThrow()

    const manifest = await readJson('client/manifest.json')
    const scripts = manifest.tac.scripts
    expect(scripts.indexOf('/shared/scripts/ThemeDefault.js')).toBeLessThan(scripts.indexOf('/shared/assets/duvay/duvay.min.js'))
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
    const footer = await readJson('client/shared/data/footer.json')
    const links = footer.flatMap((column) => column.links.map((link) => link.href))
    expect(links.length).toBeGreaterThan(5)
    const docs = JSON.parse(await read('client/shared/data/docs.json'))
    const published = ['/docs', '/docs/features', ...docs.order.map((slug) => `/docs/${slug}`)]
    for (const href of links) {
      if (href.startsWith('/docs')) expect(published).toContain(href)
    }
  })

  test('every feature is a page of its own', async () => {
    const routes = await readdir(fileURLToPath(new URL('../client/pages/docs/features', import.meta.url)))
    expect(routes).toContain('tac.html')
    expect(routes).toContain('_id')

    const catalogue = await readJson('client/shared/data/features.json')
    for (const feature of catalogue.features) {
      expect(feature.id).toMatch(/^[a-z0-9]+(-[a-z0-9]+)*$/)
    }
    const ids = catalogue.features.map((feature) => feature.id)
    expect(new Set(ids).size).toBe(ids.length)
  })

  test('the sidebar reaches every page, and the pager passes through all of them', async () => {
    const navigation = await read('client/shared/utils/DocumentationUtils.js')
    const docs = await readJson('client/shared/data/docs.json')
    const catalogue = await readJson('client/shared/data/features.json')

    expect(navigation).toContain('#catalogue().features')
    const groups = [...new Set(catalogue.features.map((feature) => feature.group))]
    for (const group of groups) expect(catalogue.groups).toContain(group)

    const total = docs.order.length + catalogue.features.length
    expect(total).toBeGreaterThan(docs.order.length)
  })

  test('the search index covers features, not only guides', async () => {
    const navigation = await read('client/shared/utils/DocumentationUtils.js')
    const search = await read('client/services/SearchService.js')
    const searchTemplate = await read('client/components/site/search/tac.html')
    expect(navigation).toContain('/docs/features/')
    expect(navigation).toContain('static searchDocs')
    expect(navigation).toContain("kind: 'section'")
    expect(navigation).toContain("kind: 'feature'")
    expect(search).toContain('this.results = DocumentationUtils.searchDocs(query)')
    expect(searchTemplate).toContain('on:input="search.find($event.target.value)"')
    expect(search).toContain('keydown')
    expect(search).toContain('this.taskScheduler.render()')
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

    const catalogue = JSON.parse(await read('client/shared/data/features.json'))
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
    const catalogue = JSON.parse(await read('client/shared/data/features.json'))
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
    const { PlatformUtils: { inReadingOrder, rank } } = await import('../client/shared/utils/PlatformUtils.js')

    expect(rank('client/services/TallyService.js')).toBeLessThan(rank('tac.html'))
    expect(rank('tac.html')).toBeLessThan(rank('tac.css'))
    expect(rank('client/services/TallyService.js')).toBeLessThan(rank('client/services/TallyService.ts'))
    expect(rank('client/services/TallyService.ts')).toBeLessThan(rank('client/services/TallyService.swift'))
    expect(rank('client/services/TallyService.rs')).toBeLessThan(rank('client/services/TallyService.go'))
    expect(rank('client/services/TallyService.go')).toBeLessThan(rank('tac.html'))
    expect(rank('emitted')).toBeGreaterThan(rank('tac.css'))

    const shuffled = [{ name: 'emitted' }, { name: 'tac.css' }, { name: 'tac.html' }, { name: 'client/services/TallyService.swift' }]
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
    const languages = JSON.parse(await read('client/shared/data/languages.json'))
    const files = languages.tac.files

    expect(files.filter((file) => file.name.endsWith('.html'))).toHaveLength(1)
    expect(files.filter((file) => file.name.endsWith('.css'))).toHaveLength(1)

    expect(isCompanion('client/components/basket/tally/tac.html')).toBe(false)
    expect(isCompanion('client/components/basket/tally/tac.css')).toBe(false)
    expect(isCompanion('client/services/TallyService.swift')).toBe(true)

    const ordered = inReadingOrder(files)
    expect(ordered.filter(file => file.name === 'client/manifest.json')).toHaveLength(1)
    const order = ordered.filter(file => file.name !== 'client/manifest.json').map((file) => file.name.split('/').pop())
    expect(order.at(-2)).toBe('tac.html')
    expect(order.at(-1)).toBe('tac.css')
    expect(order.slice(0, -2).every(name => isCompanion(name))).toBe(true)
  })

  test('a platform strip only appears where the example actually differs', async () => {
    const { PlatformUtils: { spansPlatforms, filesFor, reachOf } } =
      await import('../client/shared/utils/PlatformUtils.js')

    const service = (extension) => `client/services/TallyService.${extension}`
    const names = (files) => files.map((file) => file.name)
    expect(spansPlatforms([{ name: service('js') }])).toBe(false)
    expect(spansPlatforms([{ name: service('js') }, { name: service('go') }])).toBe(false)
    expect(spansPlatforms([{ name: service('js') }, { name: service('swift') }])).toBe(false)
    expect(spansPlatforms([{ name: service('js') }, { name: service('py') }])).toBe(true)

    const everyPlatform = ['web', 'macos', 'ios', 'android', 'windows', 'linux']
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

    const mixed = [{ name: service('py') }, { name: service('swift') }, { name: service('kt') }, { name: service('js') }]
    expect(names(filesFor(mixed, 'android'))).toEqual([service('swift'), service('kt'), service('js')])
    expect(names(filesFor(mixed, 'web'))).toEqual([service('py'), service('swift'), service('kt'), service('js')])

    const view = 'client/components/basket/tally/tac.html'
    const webAssembly = [{ name: service('rs') }, { name: service('go') }, { name: service('cs') }, { name: view }]
    expect(names(filesFor(webAssembly, 'web'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor(webAssembly, 'windows'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor(webAssembly, 'linux'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor(webAssembly, 'ios'))).toEqual([service('rs'), service('go'), service('cs'), view])
    expect(names(filesFor([{ name: service('cs') }], 'android'))).toEqual([service('cs')])
    expect(filesFor([{ name: service('py') }], 'ios')).toEqual([])
  })

  test('each home page platform tab shows every service language that can run there', async () => {
    const { PlatformUtils: { filesFor, inReadingOrder, isCompanion, languageOf } } =
      await import('../client/shared/utils/PlatformUtils.js')
    const languages = JSON.parse(await read('client/shared/data/languages.json'))
    const shownOn = (platform) => inReadingOrder(filesFor(languages.tac.files, platform))
      .filter((file) => isCompanion(file.name))
      .map((file) => languageOf(file.name))

    const browser = ['JavaScript', 'TypeScript', 'Dart', 'Java']
    const everywhere = ['C#', 'F#', 'Rust', 'Go', 'Ruby', 'Lua']
    expect(shownOn('web')).toEqual(['JavaScript', 'TypeScript', 'Dart', 'Python', 'Java', ...everywhere])
    expect(shownOn('macos')).toEqual([...browser, 'Swift', ...everywhere])
    expect(shownOn('ios')).toEqual([...browser, 'Swift', ...everywhere])
    expect(shownOn('android')).toEqual([...browser, 'Kotlin', ...everywhere])
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
    const source = await read('client/services/DrawerService.rb')
    expect(source).toContain('key == "Escape"')
  })

  test('the docs shell dissolves its wrappers where the rule can win', async () => {
    const layout = await read('client/components/docs/layout/tac.css')
    expect(layout).toContain('display: contents')
    expect(layout).toContain('data-tachyon-component')
    const site = await read('client/shared/styles/site.css')
    expect(site).not.toMatch(/^\.docs-page \{\n\s*display: contents/m)
  })
})
