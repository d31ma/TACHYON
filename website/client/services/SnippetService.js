import { PlatformUtils } from '/shared/utils/PlatformUtils.js'

/** @import { PlatformChanged } from '/models/PlatformChanged.ts' */
/** @import { SnippetFile } from '/models/SnippetFile.ts' */
/** @import { PlatformChoice } from '/models/PlatformChoice.ts' */
/** @import { LanguageChoice } from '/models/LanguageChoice.ts' */
/** @import { ShownFile } from '/models/ShownFile.ts' */
@Service
@NeedsAll
export default class SnippetService {
  static _FEWEST_CHOICES = 2
  /** @type {SnippetFile[]} */
  files
  /** @type {string} */
  axis

  /** @type {string} */
  _platform = this.#storedPlatform()
  companion = ''

  /** @returns {boolean} */
  spans() {
    return this.#spansFiles(this.#list())
  }

  /** @returns {PlatformChoice[]} */
  platforms() {
    return this.#markCurrent(PlatformUtils.list(), this.#currentPlatform())
  }

  /** @returns {LanguageChoice[]} */
  companions() {
    const companionNames = this.#languagesOf(this.#companionNames(this.#available()))
    const choices = this.#choices(companionNames, this.#chosenLanguage(companionNames))
    return this.#severalOrNone(choices)
  }

  /** @returns {ShownFile[]} */
  shown() {
    const files = this.#available()
    const hiddenNames = this.#hiddenNames(files, this.#otherLanguages(this.companions()))
    return this.#withLanguage(this.#visible(files, hiddenNames))
  }

  /**
   * @param {Event} _event
   * @param {string} name
   */
  pick(_event, name) {
    this.companion = name
  }

  /** @param {PlatformChanged} value */
  @OnEvent({"replay":true,"id":"snippet-platform"})
  platformChanged(value) {
    this._platform = value.platform
    this.companion = ''
  }

  /** @returns {SnippetFile[]} */
  #list() {
    return Array.isArray(this.files) ? this.files : []
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {boolean}
   */
  #spansFiles(files) {
    return this.axis !== 'language' && PlatformUtils.spansPlatforms(files)
  }

  /** @returns {string} */
  #currentPlatform() {
    return this._platform || this.#storedPlatform()
  }

  /**
   * @param {{ id: string, label: string }[]} platforms
   * @param {string} current
   * @returns {PlatformChoice[]}
   */
  #markCurrent(platforms, current) {
    return platforms.map((platform) => ({ ...platform, current: platform.id === current }))
  }

  /** @returns {SnippetFile[]} */
  #available() {
    const files = this.#forPlatform(this.#list(), this.spans(), this.#currentPlatform())
    return PlatformUtils.inReadingOrder(this.#withCode(files))
  }

  /**
   * @param {SnippetFile[]} files
   * @param {boolean} spans
   * @param {string} platform
   * @returns {SnippetFile[]}
   */
  #forPlatform(files, spans, platform) {
    if (!spans) return files
    return PlatformUtils.filesFor(files, platform)
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {SnippetFile[]}
   */
  #withCode(files) {
    return files.filter((file) => file.code.trim() !== '')
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {string[]}
   */
  #companionNames(files) {
    return files.map(this.#nameOf).filter(PlatformUtils.isCompanion)
  }

  /**
   * @param {string[]} names
   * @returns {string[]}
   */
  #languagesOf(names) {
    return [...new Set(names.map(PlatformUtils.languageLabel))]
  }

  /**
   * @param {string[]} companionNames
   * @returns {string}
   */
  #chosenLanguage(companionNames) {
    return companionNames.includes(this.companion) ? this.companion : companionNames[0]
  }

  /**
   * @param {string[]} companionNames
   * @param {string} chosen
   * @returns {LanguageChoice[]}
   */
  #choices(companionNames, chosen) {
    return companionNames.map((language) => ({ name: language, label: language, current: language === chosen }))
  }

  /**
   * @param {LanguageChoice[]} choices
   * @returns {LanguageChoice[]}
   */
  #severalOrNone(choices) {
    if (choices.length < SnippetService._FEWEST_CHOICES) return []
    return choices
  }

  /**
   * @param {LanguageChoice[]} companions
   * @returns {string[]}
   */
  #otherLanguages(companions) {
    return companions.filter((entry) => !entry.current).map(this.#nameOf)
  }

  /**
   * @param {SnippetFile[]} files
   * @param {string[]} hiddenLanguages
   * @returns {string[]}
   */
  #hiddenNames(files, hiddenLanguages) {
    return files.map(this.#nameOf).filter(PlatformUtils.isCompanion).filter((name) => hiddenLanguages.includes(PlatformUtils.languageLabel(name)))
  }

  /**
   * @param {SnippetFile[]} files
   * @param {string[]} hiddenNames
   * @returns {SnippetFile[]}
   */
  #visible(files, hiddenNames) {
    return files.filter((file) => !hiddenNames.includes(file.name))
  }

  /**
   * @param {{ name: string }} item
   * @returns {string}
   */
  #nameOf(item) {
    return item.name
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {ShownFile[]}
   */
  #withLanguage(files) {
    return files.map((file) => ({ ...file, language: PlatformUtils.languageLabel(file.name) }))
  }

  /** @returns {string} */
  #storedPlatform() {
    return PlatformUtils.knownPlatform(this.#savedPlatform(), PlatformUtils.detectPlatform(navigator.userAgent))
  }

  /** @returns {string | null} */
  #savedPlatform() {
    try {
      return localStorage.getItem('tachyon.platform')
    } catch {
      return null
    }
  }
}
