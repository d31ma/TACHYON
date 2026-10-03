import documents from '/shared/data/docs.json' with { type: 'json' }
import features from '/shared/data/features.json' with { type: 'json' }

/** @import { DocumentationCatalogue } from '/models/DocumentationCatalogue.ts' */
/** @import { FeatureCatalogue } from '/models/FeatureCatalogue.ts' */
/** @import { Guide } from '/models/Guide.ts' */
/** @import { Section } from '/models/Section.ts' */
/** @import { SnippetFile } from '/models/SnippetFile.ts' */
/** @import { SearchResult } from '/models/SearchResult.ts' */
/** @import { SearchRow } from '/models/SearchRow.ts' */
/** @import { SearchHit } from '/models/SearchHit.ts' */
/** @import { GuideLink } from '/models/GuideLink.ts' */
/** @import { FeatureEntry } from '/models/FeatureEntry.ts' */
export class DocumentationUtils {
  /** @param {string} value @returns {string} */
  static slugify(value) {
    return DocumentationUtils.#slug(value)
  }

  /** @param {string} slug @returns {Guide | undefined} */
  static guide(slug) {
    return DocumentationUtils.#documents().topics[slug]
  }

  /** @param {string} slug @param {string} hash @returns {string | undefined} */
  static redirect(slug, hash = '') {
    const target = DocumentationUtils.#documents().redirects?.[slug]
    const anchor = hash.replace(/^#/, '')
    const holder = DocumentationUtils.#holderFor(anchor)
    return DocumentationUtils.#redirectTo(target, holder, hash)
  }

  /** @param {string} query @param {number} maximum @returns {SearchResult[]} */
  static searchDocs(query, maximum = 12) {
    const term = DocumentationUtils.#searchTerm(query)
    const rows = DocumentationUtils.#searchRows()
    const hits = DocumentationUtils.#hits(rows, term)
    return DocumentationUtils.#bestHits(hits, maximum)
  }

  /** @returns {GuideLink[]} */
  static guides() {
    return DocumentationUtils.#guideLinks()
  }

  /** @returns {(FeatureEntry & { path: string })[]} */
  static features() {
    return DocumentationUtils.#featureLinks()
  }

  /** @param {string} value @returns {string} */
  static #slug(value) {
    return String(value).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')
  }

  /** @returns {GuideLink[]} */
  static #guideLinks() {
    return DocumentationUtils.#documents().order.map(DocumentationUtils.#guideLink)
  }

  /** @param {string} slug @returns {GuideLink} */
  static #guideLink(slug) {
    const entry = DocumentationUtils.#documents().topics[slug]
    return { slug, title: entry.title, summary: entry.summary, path: `/docs/${slug}` }
  }

  /** @param {string} anchor @returns {string | undefined} */
  static #holderFor(anchor) {
    return DocumentationUtils.#anchorHolder(DocumentationUtils.#matchingGuides(anchor), anchor)
  }

  /** @param {string} anchor @returns {string[]} */
  static #matchingGuides(anchor) {
    return DocumentationUtils.#documents().order.filter((slug) => DocumentationUtils.#hasAnchor(slug, anchor))
  }

  /** @param {string} slug @param {string} anchor @returns {boolean} */
  static #hasAnchor(slug, anchor) {
    return DocumentationUtils.#documents().topics[slug].sections.some((section) => DocumentationUtils.slugify(section.heading) === anchor)
  }

  /** @param {string[]} matching @param {string} anchor @returns {string | undefined} */
  static #anchorHolder(matching, anchor) {
    return anchor ? matching[0] : undefined
  }

  /** @param {string | undefined} target @param {string | undefined} holder @param {string} hash @returns {string | undefined} */
  static #redirectTo(target, holder, hash) {
    const destination = DocumentationUtils.#destination(holder, target)
    return DocumentationUtils.#whenRedirected(target, destination, hash)
  }

  /** @param {string | undefined} holder @param {string | undefined} target @returns {string | undefined} */
  static #destination(holder, target) {
    return holder ?? target
  }

  /** @param {string | undefined} target @param {string | undefined} destination @param {string} hash @returns {string | undefined} */
  static #whenRedirected(target, destination, hash) {
    return target ? `/docs/${destination}${hash}` : undefined
  }

  /** @returns {(FeatureEntry & { path: string })[]} */
  static #featureLinks() {
    return DocumentationUtils.#catalogue().features.map(DocumentationUtils.#featureLink)
  }

  /** @param {FeatureEntry} feature @returns {FeatureEntry & { path: string }} */
  static #featureLink(feature) {
    return { ...feature, path: `/docs/features/${feature.id}` }
  }

  /** @param {string} query @returns {string} */
  static #searchTerm(query) {
    return String(query ?? '').trim().toLowerCase()
  }

  /** @returns {SearchRow[]} */
  static #searchRows() {
    return [...DocumentationUtils.#topicRows(), ...DocumentationUtils.#featureRows()]
  }

  /** @returns {SearchRow[]} */
  static #topicRows() {
    return DocumentationUtils.guides().flatMap(DocumentationUtils.#rowsForGuide)
  }

  /** @param {GuideLink} entry @returns {SearchRow[]} */
  static #rowsForGuide(entry) {
    const topic = DocumentationUtils.#documents().topics[entry.slug]
    const rows = DocumentationUtils.#sectionRows(entry, DocumentationUtils.#sectionsOf(topic))
    return [{ title: entry.title, path: entry.path, kind: 'topic', text: entry.summary }, ...rows]
  }

  /** @param {Guide} topic @returns {Section[]} */
  static #sectionsOf(topic) {
    return topic.sections ?? []
  }

  /** @param {GuideLink} entry @param {Section[]} sections @returns {SearchRow[]} */
  static #sectionRows(entry, sections) {
    return sections.map((section) => DocumentationUtils.#sectionRow(entry, section))
  }

  /** @param {GuideLink} entry @param {Section & { files?: SnippetFile[] }} section @returns {SearchRow} */
  static #sectionRow(entry, section) {
    const anchor = DocumentationUtils.slugify(section.heading)
    const code = DocumentationUtils.#codeOf(section.files)
    return {
      title: `${entry.title} · ${section.heading}`,
      path: `${entry.path}#${anchor}`,
      kind: 'section',
      text: DocumentationUtils.#joinText([section.summary, section.body, section.code, ...code]),
    }
  }

  /** @param {SnippetFile[] | undefined} files @returns {string[]} */
  static #codeOf(files) {
    return DocumentationUtils.#knownFiles(files).map(DocumentationUtils.#fileCode)
  }

  /** @param {SnippetFile[] | undefined} files @returns {SnippetFile[]} */
  static #knownFiles(files) {
    return files ?? []
  }

  /** @param {SnippetFile} file @returns {string} */
  static #fileCode(file) {
    return file.code
  }

  /** @param {(string | undefined)[]} parts @returns {string} */
  static #joinText(parts) {
    return parts.filter(Boolean).join('\n')
  }

  /** @returns {SearchRow[]} */
  static #featureRows() {
    return DocumentationUtils.features().map(DocumentationUtils.#featureRow)
  }

  /** @param {FeatureEntry & { path: string }} entry @returns {SearchRow} */
  static #featureRow(entry) {
    return {
      title: entry.title, path: entry.path, kind: 'feature',
      text: DocumentationUtils.#joinText([entry.summary, entry.group, ...DocumentationUtils.#codeOf(entry.files)]),
    }
  }

  /** @param {SearchRow[]} rows @param {string} term @returns {SearchHit[]} */
  static #hits(rows, term) {
    const hits = DocumentationUtils.#ranked(rows, term)
    return DocumentationUtils.#searchable(term.length, hits)
  }

  /** @param {SearchRow[]} rows @param {string} term @returns {SearchHit[]} */
  static #ranked(rows, term) {
    return rows.map((row) => DocumentationUtils.#hit(row, term)).filter(DocumentationUtils.#matched)
  }

  /** @param {SearchRow} row @param {string} term @returns {SearchHit} */
  static #hit(row, term) {
    const title = row.title.toLowerCase()
    const text = row.text.toLowerCase()
    const priorities = [[title.startsWith(term), 3], [title.includes(term), 2], [text.includes(term), 1]]
    return { row, rank: DocumentationUtils.#rankOf(priorities) }
  }

  /** @param {(boolean | number)[][]} priorities @returns {number} */
  static #rankOf(priorities) {
    return Number(priorities.find(DocumentationUtils.#firstTrue)?.[1] ?? 0)
  }

  /** @param {(boolean | number)[]} priority @returns {boolean} */
  static #firstTrue(priority) {
    return Boolean(priority[0])
  }

  /** @param {SearchHit} hit @returns {boolean} */
  static #matched(hit) {
    return hit.rank > 0
  }

  /** @param {number} length @param {SearchHit[]} hits @returns {SearchHit[]} */
  static #searchable(length, hits) {
    return length < 2 ? [] : hits
  }

  /** @param {SearchHit[]} hits @param {number} maximum @returns {SearchResult[]} */
  static #bestHits(hits, maximum) {
    return hits.sort(DocumentationUtils.#compareHits).slice(0, maximum).map(DocumentationUtils.#resultOf)
  }

  /** @param {SearchHit} left @param {SearchHit} right @returns {number} */
  static #compareHits(left, right) {
    return right.rank - left.rank || left.row.title.length - right.row.title.length
  }

  /** @param {SearchHit} hit @returns {SearchResult} */
  static #resultOf(hit) {
    return { title: hit.row.title, path: hit.row.path, kind: hit.row.kind }
  }

  /** @returns {DocumentationCatalogue} */
  static #documents() {
    return /** @type {DocumentationCatalogue} */ (documents)
  }

  /** @returns {FeatureCatalogue} */
  static #catalogue() {
    return /** @type {FeatureCatalogue} */ (features)
  }
}
