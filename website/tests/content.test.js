import { describe, expect, test } from 'bun:test'
import { readFileSync, readdirSync } from 'node:fs'
import path from 'node:path'
import { SiteContent } from '../scripts/site-content.mjs'

class ContentFiles {
  /** @returns {string[]} */
  static onDisk() {
    const entries = readdirSync(SiteContent.contentFolder, { recursive: true, withFileTypes: true })
    const files = entries.filter(entry => entry.isFile() && !entry.name.startsWith('.'))
    return files.map(entry => path.join(entry.parentPath, entry.name))
  }

  /** @returns {Set<string>} */
  static referenced() {
    const files = new Set([SiteContent.listFile])
    for (const generated of SiteContent.generatedFiles()) {
      for (const file of SiteContent.referencedFiles(path.join(SiteContent.contentFolder, generated.source))) files.add(file)
    }
    return files
  }

  /**
   * @param {string} file
   * @returns {string}
   */
  static readOrEmpty(file) {
    try {
      return readFileSync(file, 'utf8')
    } catch {
      return ''
    }
  }
}

describe('site data is generated from website/content', () => {
  test('every generated file matches website/content; edit website/content, then run bun run content', () => {
    const staleFiles = SiteContent.generatedFiles()
      .filter(file => ContentFiles.readOrEmpty(SiteContent.outputPath(file)) !== SiteContent.render(file))
      .map(file => file.output)
    expect(staleFiles).toEqual([])
  })

  test('every file under website/content is part of a generated file', () => {
    const referenced = ContentFiles.referenced()
    const unused = ContentFiles.onDisk().filter(file => !referenced.has(file))
    expect(unused.map(file => path.relative(SiteContent.contentFolder, file))).toEqual([])
  })
})

describe('the docs guide reads in order', () => {
  test('the reading order is the groups, in order, and every topic is in one group', () => {
    /** @type {{ order: string[], groups: { topics: string[] }[], topics: Record<string, { before?: string[] }>, redirects?: Record<string, string> }} */
    const docs = SiteContent.data('docs.json')
    expect(docs.order).toEqual(docs.groups.flatMap(group => group.topics))
    expect(new Set(docs.order).size).toBe(docs.order.length)
    expect(Object.keys(docs.topics).sort()).toEqual([...docs.order].sort())
  })

  test('every "before" link and every old URL leads to a topic', () => {
    /** @type {{ order: string[], groups: { topics: string[] }[], topics: Record<string, { before?: string[] }>, redirects?: Record<string, string> }} */
    const docs = SiteContent.data('docs.json')
    for (const topic of Object.values(docs.topics)) {
      for (const slug of topic.before ?? []) expect(docs.topics[slug]).toBeDefined()
    }
    for (const [old, target] of Object.entries(docs.redirects ?? {})) {
      expect(docs.topics[old]).toBeUndefined()
      expect(docs.topics[target]).toBeDefined()
    }
  })
})
