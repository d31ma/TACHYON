import { describe, expect, test } from 'bun:test'
import { readFile } from 'node:fs/promises'
import { plugin } from 'bun'
import { fileURLToPath } from 'node:url'
import { PlatformUtils } from '../client/shared/utils/PlatformUtils.js'

plugin({
  name: 'tachyon-website-shared-json',
  setup(build) {
    build.onResolve({ filter: /^\/shared\/data\/(?:docs|features)\.json$/ }, ({ path, importer }) => {
      if (!importer.endsWith('/DocumentationUtils.js')) return
      return { path: fileURLToPath(new URL(`../client${path}`, import.meta.url)) }
    })
  },
})

describe('stateless platform utilities', () => {
  test('platform choices and reach cannot retain caller mutations', () => {
    const choices = PlatformUtils.list()
    choices[0].id = 'changed'
    const reach = /** @type {string[]} */ (PlatformUtils.reachOf('client/services/DisplayService.js'))
    reach[0] = 'changed'
    expect(PlatformUtils.list()[0].id).toBe('web')
    expect(PlatformUtils.reachOf('client/services/DisplayService.js')?.[0]).toBe('web')
  })

  test('language and platform filtering preserve server and native reach', () => {
    const files = [{ name: 'client/services/ViewService.swift', code: '' }, { name: 'client/services/ViewService.js', code: '' }, { name: 'server/controllers/ViewController.js', code: '' }]
    expect(PlatformUtils.filesFor(files, 'web').map(file => file.name)).toEqual(['client/services/ViewService.js', 'server/controllers/ViewController.js'])
    expect(PlatformUtils.reachOf(files[2].name)).toBeNull()
    expect(PlatformUtils.spansPlatforms(files)).toBe(true)
    expect(PlatformUtils.isCompanion('tac.html')).toBe(false)
    expect(PlatformUtils.isCompanion('client/services/ViewService.swift')).toBe(true)
    expect(PlatformUtils.languageOf(files[0].name)).toBe('Swift')
    expect(PlatformUtils.languageLabel('unknown')).toBe('')
  })

  test('reading order and platform selection retain their original contracts', () => {
    expect(PlatformUtils.rank('ViewService.js')).toBeLessThan(PlatformUtils.rank('tac.html'))
    expect(PlatformUtils.inReadingOrder([{ name: 'tac.css', code: '' }, { name: 'tac.html', code: '' }, { name: 'ViewService.js', code: '' }]).map(file => file.name)).toEqual(['ViewService.js', 'tac.html', 'tac.css'])
    expect(PlatformUtils.detectPlatform('Mozilla Macintosh')).toBe('macos')
    expect(PlatformUtils.detectPlatform('iPhone')).toBe('ios')
    expect(PlatformUtils.knownPlatform('android', 'web')).toBe('android')
    expect(PlatformUtils.knownPlatform('unrecognized', 'web')).toBe('web')
  })
})

describe('actual documentation source with local JSON imports', () => {
  test('guide links, slugs and redirects still resolve the published data', async () => {
    const { DocumentationUtils } = await import('../client/shared/utils/DocumentationUtils.js')
    const documents = JSON.parse(await readFile(new URL('../client/shared/data/docs.json', import.meta.url), 'utf8'))
    expect(DocumentationUtils.slugify(' On Start! ')).toBe('on-start')
    expect(DocumentationUtils.guides().map(entry => entry.slug)).toEqual(documents.order)
    expect(DocumentationUtils.guide(documents.order[0])?.title).toBe(documents.topics[documents.order[0]].title)
    expect(DocumentationUtils.guide('missing')).toBeUndefined()
    expect(DocumentationUtils.redirect('missing')).toBeUndefined()
    const [slug, target] = Object.entries(documents.redirects)[0]
    expect(DocumentationUtils.redirect(slug)).toBe(`/docs/${target}`)
  })

  test('search covers guide sections and feature pages with the same limit', async () => {
    const { DocumentationUtils } = await import('../client/shared/utils/DocumentationUtils.js')
    expect(DocumentationUtils.searchDocs('r')).toEqual([])
    expect(DocumentationUtils.searchDocs('routing', 2)).toHaveLength(2)
    expect(DocumentationUtils.searchDocs('compiler').some(entry => entry.kind === 'feature')).toBe(true)
    expect(DocumentationUtils.searchDocs('browser', 100).some(entry => entry.kind === 'section')).toBe(true)
    expect(DocumentationUtils.features().every(entry => entry.path === `/docs/features/${entry.id}`)).toBe(true)
  })

  test('search reads a section summary', async () => {
    const { DocumentationUtils } = await import('../client/shared/utils/DocumentationUtils.js')
    const hits = DocumentationUtils.searchDocs('Tachyon passes one when it creates the service', 100)
    expect(hits.map(entry => entry.path)).toContain('/docs/annotations#lifecycle-and-events-tac')
  })
})
