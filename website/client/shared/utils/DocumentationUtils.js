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
  static slugOf(value) {
    return String(value).toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '')
  }

  /** @param {string} slug @returns {Guide | undefined} */
  static guide(slug) {
    return DocumentationUtils._documents().topics[slug]
  }

  /** @param {string} slug @param {string} hash @returns {string | undefined} */
  static redirect(slug, hash = '') {
    const target = DocumentationUtils._documents().redirects?.[slug]
    const anchor = hash.replace(/^#/, '')
    const holder = DocumentationUtils._holderFor(anchor)
    return DocumentationUtils._redirectTo(target, holder, hash)
  }

  /** @param {string} query @param {number} maximum @returns {SearchResult[]} */
  static search(query, maximum = 12) {
    const term = DocumentationUtils._searchTerm(query)
    const rows = DocumentationUtils._searchRows()
    const hits = DocumentationUtils._hits(rows, term)
    return DocumentationUtils._bestHits(hits, maximum)
  }

  /** @returns {GuideLink[]} */
  static _guideLinks() {
    return DocumentationUtils._documents().order.map(DocumentationUtils._guideLink)
  }

  /** @param {string} slug @returns {GuideLink} */
  static _guideLink(slug) {
    const entry = DocumentationUtils._documents().topics[slug]
    return { slug, title: entry.title, summary: entry.summary, path: `/docs/${slug}` }
  }

  /** @param {string} anchor @returns {string | undefined} */
  static _holderFor(anchor) {
    return DocumentationUtils._anchorHolder(DocumentationUtils._matchingGuides(anchor), anchor)
  }

  /** @param {string} anchor @returns {string[]} */
  static _matchingGuides(anchor) {
    return DocumentationUtils._documents().order.filter((slug) => DocumentationUtils._hasAnchor(slug, anchor))
  }

  /** @param {string} slug @param {string} anchor @returns {boolean} */
  static _hasAnchor(slug, anchor) {
    return DocumentationUtils._documents().topics[slug].sections.some((section) => DocumentationUtils.slugOf(section.heading) === anchor)
  }

  /** @param {string[]} matching @param {string} anchor @returns {string | undefined} */
  static _anchorHolder(matching, anchor) {
    return anchor ? matching[0] : undefined
  }

  /** @param {string | undefined} target @param {string | undefined} holder @param {string} hash @returns {string | undefined} */
  static _redirectTo(target, holder, hash) {
    const destination = DocumentationUtils._destination(holder, target)
    return DocumentationUtils._whenRedirected(target, destination, hash)
  }

  /** @param {string | undefined} holder @param {string | undefined} target @returns {string | undefined} */
  static _destination(holder, target) {
    return holder ?? target
  }

  /** @param {string | undefined} target @param {string | undefined} destination @param {string} hash @returns {string | undefined} */
  static _whenRedirected(target, destination, hash) {
    return target ? `/docs/${destination}${hash}` : undefined
  }

  /** @returns {(FeatureEntry & { path: string })[]} */
  static _featureLinks() {
    return DocumentationUtils._catalogue().features.map(DocumentationUtils._featureLink)
  }

  /** @param {FeatureEntry} feature @returns {FeatureEntry & { path: string }} */
  static _featureLink(feature) {
    return { ...feature, path: `/docs/features/${feature.id}` }
  }

  /** @param {string} query @returns {string} */
  static _searchTerm(query) {
    return String(query ?? '').trim().toLowerCase()
  }

  /** @returns {SearchRow[]} */
  static _searchRows() {
    return [...DocumentationUtils._topicRows(), ...DocumentationUtils._featureRows()]
  }

  /** @returns {SearchRow[]} */
  static _topicRows() {
    return DocumentationUtils._guideLinks().flatMap(DocumentationUtils._rowsForGuide)
  }

  /** @param {GuideLink} entry @returns {SearchRow[]} */
  static _rowsForGuide(entry) {
    const topic = DocumentationUtils._documents().topics[entry.slug]
    const rows = DocumentationUtils._sectionRows(entry, DocumentationUtils._sectionsOf(topic))
    return [{ title: entry.title, path: entry.path, kind: 'topic', text: entry.summary }, ...rows]
  }

  /** @param {Guide} topic @returns {Section[]} */
  static _sectionsOf(topic) {
    return topic.sections ?? []
  }

  /** @param {GuideLink} entry @param {Section[]} sections @returns {SearchRow[]} */
  static _sectionRows(entry, sections) {
    return sections.map((section) => DocumentationUtils._sectionRow(entry, section))
  }

  /** @param {GuideLink} entry @param {Section & { files?: SnippetFile[] }} section @returns {SearchRow} */
  static _sectionRow(entry, section) {
    const anchor = DocumentationUtils.slugOf(section.heading)
    const code = DocumentationUtils._codeOf(section.files)
    return {
      title: `${entry.title} · ${section.heading}`,
      path: `${entry.path}#${anchor}`,
      kind: 'section',
      text: DocumentationUtils._joinText([section.summary, section.body, section.code, ...code]),
    }
  }

  /** @param {SnippetFile[] | undefined} files @returns {string[]} */
  static _codeOf(files) {
    return DocumentationUtils._knownFiles(files).map(DocumentationUtils._fileCode)
  }

  /** @param {SnippetFile[] | undefined} files @returns {SnippetFile[]} */
  static _knownFiles(files) {
    return files ?? []
  }

  /** @param {SnippetFile} file @returns {string} */
  static _fileCode(file) {
    return file.code
  }

  /** @param {(string | undefined)[]} parts @returns {string} */
  static _joinText(parts) {
    return parts.filter(Boolean).join('\n')
  }

  /** @returns {SearchRow[]} */
  static _featureRows() {
    return DocumentationUtils._featureLinks().map(DocumentationUtils._featureRow)
  }

  /** @param {FeatureEntry & { path: string }} entry @returns {SearchRow} */
  static _featureRow(entry) {
    return {
      title: entry.title, path: entry.path, kind: 'feature',
      text: DocumentationUtils._joinText([entry.summary, entry.group, ...DocumentationUtils._codeOf(entry.files)]),
    }
  }

  /** @param {SearchRow[]} rows @param {string} term @returns {SearchHit[]} */
  static _hits(rows, term) {
    const hits = DocumentationUtils._ranked(rows, term)
    return DocumentationUtils._searchable(term.length, hits)
  }

  /** @param {SearchRow[]} rows @param {string} term @returns {SearchHit[]} */
  static _ranked(rows, term) {
    return rows.map((row) => DocumentationUtils._hit(row, term)).filter(DocumentationUtils._matched)
  }

  /** @param {SearchRow} row @param {string} term @returns {SearchHit} */
  static _hit(row, term) {
    return { row, rank: DocumentationUtils._rankOf(row.title.toLowerCase(), row.text.toLowerCase(), term) }
  }

  /** @param {string} title @param {string} text @param {string} term @returns {number} */
  static _rankOf(title, text, term) {
    if (title.startsWith(term)) {
      return 3
    } else if (title.includes(term)) {
      return 2
    } else if (text.includes(term)) {
      return 1
    }
    return 0
  }

  /** @param {SearchHit} hit @returns {boolean} */
  static _matched(hit) {
    return hit.rank > 0
  }

  /** @param {number} length @param {SearchHit[]} hits @returns {SearchHit[]} */
  static _searchable(length, hits) {
    return length < DocumentationUtils.shortestTerm() ? [] : hits
  }

  /** @param {SearchHit[]} hits @param {number} maximum @returns {SearchResult[]} */
  static _bestHits(hits, maximum) {
    return hits.sort(DocumentationUtils._compareHits).slice(0, maximum).map(DocumentationUtils._resultOf)
  }

  /** @param {SearchHit} left @param {SearchHit} right @returns {number} */
  static _compareHits(left, right) {
    return right.rank - left.rank || left.row.title.length - right.row.title.length
  }

  /** @param {SearchHit} hit @returns {SearchResult} */
  static _resultOf(hit) {
    return { title: hit.row.title, path: hit.row.path, kind: hit.row.kind }
  }

  /** @returns {number} */
  static shortestTerm() {
    return 2
  }

  /** @returns {DocumentationCatalogue} */
  static _documents() {
    return /** @type {DocumentationCatalogue} */ (documents)
  }

  /** @returns {FeatureCatalogue} */
  static _catalogue() {
    return /** @type {FeatureCatalogue} */ (features)
  }
}
