import { currentPath, currentSlug, feature, slugify } from '/shared/scripts/navigation.js'

/** @import { FeatureEntry } from '/models/FeatureEntry.ts' */
/** @import { SnippetFile } from '/models/SnippetFile.ts' */
@Service
export default class FeatureService {
  /** @type {FeatureEntry} */
  static _NOT_FOUND = {
    id: '',
    title: 'Feature not found',
    summary: 'That feature does not exist. Open the list of features instead.',
    group: '',
    files: [],
  }

  /** @returns {string} */
  get id() {
    return currentSlug()
  }

  /** @returns {string} */
  path() {
    return currentPath()
  }

  /** @returns {string} */
  get source() {
    return this.#sourceOf(feature(this.id))
  }

  /** @returns {FeatureEntry} */
  entry() {
    return this.#orNotFound(feature(this.id))
  }

  /** @returns {string} */
  groupAnchor() {
    return `/docs/features#${slugify(this.entry().group)}`
  }

  /** @returns {SnippetFile[]} */
  files() {
    return this.#filesOf(this.entry())
  }

  @OnStart
  initialize() {
    document.title = `${this.entry().title} — Tachyon docs`
  }

  /**
   * @param {FeatureEntry | undefined} found
   * @returns {string}
   */
  #sourceOf(found) {
    if (!found) return 'website/content/features/page.json'
    return `website/content/features/${this.id}/feature.json`
  }

  /**
   * @param {FeatureEntry | undefined} found
   * @returns {FeatureEntry}
   */
  #orNotFound(found) {
    return found ?? FeatureService._NOT_FOUND
  }

  /**
   * @param {FeatureEntry} entry
   * @returns {SnippetFile[]}
   */
  #filesOf(entry) {
    return entry.files ?? []
  }
}
