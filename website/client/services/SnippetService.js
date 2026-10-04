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
  static _PLATFORM_KEY = 'tachyon.platform'
  /** @type {SnippetFile[]} */
  files
  /** @type {string} */
  axis
  /** @type {PreferenceStrings} */
  @Needs
  preferenceStrings

  /** @type {string} */
  _platform = ''
  companion = ''

  /** @returns {boolean} */
  spans() {
    return this._spansFiles(this._list())
  }

  /** @returns {PlatformChoice[]} */
  platforms() {
    return this._markCurrent(PlatformUtils.list(), this._currentPlatform())
  }

  /** @returns {LanguageChoice[]} */
  companions() {
    const companionNames = this._languagesOf(this._companionNames(this._available()))
    const choices = this._choices(companionNames, this._chosenLanguage(companionNames))
    return this._severalOrNone(choices)
  }

  /** @returns {ShownFile[]} */
  shown() {
    const files = this._available()
    const hiddenNames = this._hiddenNames(files, this._otherLanguages(this.companions()))
    return this._withLanguage(this._visible(files, hiddenNames))
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
  _list() {
    return Array.isArray(this.files) ? this.files : []
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {boolean}
   */
  _spansFiles(files) {
    return this.axis !== 'language' && PlatformUtils.spansPlatforms(files)
  }

  /** @returns {string} */
  _currentPlatform() {
    return this._platform || this._storedPlatform()
  }

  /**
   * @param {{ id: string, label: string }[]} platforms
   * @param {string} current
   * @returns {PlatformChoice[]}
   */
  _markCurrent(platforms, current) {
    return platforms.map((platform) => ({ ...platform, current: platform.id === current }))
  }

  /** @returns {SnippetFile[]} */
  _available() {
    const files = this._forPlatform(this._list(), this.spans(), this._currentPlatform())
    return PlatformUtils.inReadingOrder(this._withCode(files))
  }

  /**
   * @param {SnippetFile[]} files
   * @param {boolean} spans
   * @param {string} platform
   * @returns {SnippetFile[]}
   */
  _forPlatform(files, spans, platform) {
    if (!spans) return files
    return PlatformUtils.filesFor(files, platform)
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {SnippetFile[]}
   */
  _withCode(files) {
    return files.filter((file) => file.code.trim() !== '')
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {string[]}
   */
  _companionNames(files) {
    return files.map(this._nameOf).filter(PlatformUtils.isCompanion)
  }

  /**
   * @param {string[]} names
   * @returns {string[]}
   */
  _languagesOf(names) {
    return [...new Set(names.map(PlatformUtils.languageLabel))]
  }

  /**
   * @param {string[]} companionNames
   * @returns {string}
   */
  _chosenLanguage(companionNames) {
    return companionNames.includes(this.companion) ? this.companion : companionNames[0]
  }

  /**
   * @param {string[]} companionNames
   * @param {string} chosen
   * @returns {LanguageChoice[]}
   */
  _choices(companionNames, chosen) {
    return companionNames.map((language) => ({ name: language, label: language, current: language === chosen }))
  }

  /**
   * @param {LanguageChoice[]} choices
   * @returns {LanguageChoice[]}
   */
  _severalOrNone(choices) {
    if (choices.length < SnippetService._FEWEST_CHOICES) return []
    return choices
  }

  /**
   * @param {LanguageChoice[]} companions
   * @returns {string[]}
   */
  _otherLanguages(companions) {
    return companions.filter((entry) => !entry.current).map(this._nameOf)
  }

  /**
   * @param {SnippetFile[]} files
   * @param {string[]} hiddenLanguages
   * @returns {string[]}
   */
  _hiddenNames(files, hiddenLanguages) {
    return files.map(this._nameOf).filter(PlatformUtils.isCompanion).filter((name) => hiddenLanguages.includes(PlatformUtils.languageLabel(name)))
  }

  /**
   * @param {SnippetFile[]} files
   * @param {string[]} hiddenNames
   * @returns {SnippetFile[]}
   */
  _visible(files, hiddenNames) {
    return files.filter((file) => !hiddenNames.includes(file.name))
  }

  /**
   * @param {{ name: string }} item
   * @returns {string}
   */
  _nameOf(item) {
    return item.name
  }

  /**
   * @param {SnippetFile[]} files
   * @returns {ShownFile[]}
   */
  _withLanguage(files) {
    return files.map((file) => ({ ...file, language: PlatformUtils.languageLabel(file.name) }))
  }

  /** @returns {string} */
  _storedPlatform() {
    const saved = this.preferenceStrings.read(SnippetService._PLATFORM_KEY)
    return PlatformUtils.knownPlatform(saved, PlatformUtils.detectPlatform(navigator.userAgent))
  }
}
