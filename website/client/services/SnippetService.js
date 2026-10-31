import { PlatformUtils } from '/shared/utils/PlatformUtils.js'

/** @import { PlatformChanged } from '/models/PlatformChanged.ts' */
/** @import { SnippetFile } from '/models/SnippetFile.ts' */
/** @import { PlatformChoice } from '/models/PlatformChoice.ts' */
/** @import { LanguageChoice } from '/models/LanguageChoice.ts' */
/** @import { ShownFile } from '/models/ShownFile.ts' */
/** @import { TaskScheduler } from '/models/TaskScheduler.ts' */
@Service
@NeedsAll
export default class SnippetService {
  static _FEWEST_CHOICES = 2
  static _PLATFORM_KEY = 'tachyon.platform'
  static _LANGUAGE = 'language'
  static _PLATFORM = 'platform'
  static _TAB_PANEL = 'tabpanel'
  /** @type {Map<string, (index: number, count: number) => number>} */
  static _MOVES = new Map([
    ['ArrowRight', (/** @type {number} */ index, /** @type {number} */ count) => (index + 1) % count],
    ['ArrowLeft', (/** @type {number} */ index, /** @type {number} */ count) => (index - 1 + count) % count],
    ['Home', () => 0],
    ['End', (/** @type {number} */ _index, /** @type {number} */ count) => count - 1],
  ])
  static _made = 0
  /** @type {SnippetFile[]} */
  files
  /** @type {string} */
  axis
  /** @type {PreferenceStrings} */
  @Needs
  preferenceStrings
  /** @type {TaskScheduler} */
  @Needs
  taskScheduler

  /** @type {string} */
  _platform = ''
  companion = ''
  _focusing = ''
  /** @type {string} */
  _panel = SnippetService._nextPanel()

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
    void this._focusAfterRender()
  }

  /** @returns {string} */
  panelId() {
    return this._panel
  }

  /**
   * @param {string} id
   * @returns {string}
   */
  platformTabId(id) {
    return this._tabId(SnippetService._PLATFORM, id)
  }

  /**
   * @param {string} name
   * @returns {string}
   */
  languageTabId(name) {
    return this._tabId(SnippetService._LANGUAGE, name)
  }

  /** @returns {string | null} */
  panelRole() {
    return this._roleFor(this._selectedTabIds(this._platformTabIds(this.spans(), this._currentPlatform()), this.companions()))
  }

  /** @returns {string | null} */
  panelLabels() {
    return this._labelsFor(this._selectedTabIds(this._platformTabIds(this.spans(), this._currentPlatform()), this.companions()))
  }

  /**
   * @param {KeyboardEvent} event
   * @param {string} key
   */
  moveTab(event, key) {
    const current = /** @type {HTMLElement} */ (event.target)
    const tabs = this._tabsBeside(current)
    this._choose(event, this._nextTab(tabs, tabs.indexOf(current), SnippetService._MOVES.get(key)))
  }

  /** @param {PlatformChanged} value */
  @OnEvent({"replay":true,"id":"snippet-platform"})
  platformChanged(value) {
    this._platform = value.platform
    this.companion = ''
    void this._focusAfterRender()
  }

  /** @returns {string} */
  static _nextPanel() {
    SnippetService._made += 1
    return `snippet-${SnippetService._made}`
  }

  /**
   * @param {string} kind
   * @param {string} name
   * @returns {string}
   */
  _tabId(kind, name) {
    return `${this._panel}-${kind}-${encodeURIComponent(name).replaceAll('%', '-')}`
  }

  /**
   * @param {boolean} spans
   * @param {string} platform
   * @returns {string[]}
   */
  _platformTabIds(spans, platform) {
    if (!spans) return []
    return [this._tabId(SnippetService._PLATFORM, platform)]
  }

  /**
   * @param {string[]} platformIds
   * @param {LanguageChoice[]} companions
   * @returns {string[]}
   */
  _selectedTabIds(platformIds, companions) {
    const languageIds = companions.filter((entry) => entry.current).map((entry) => this._tabId(SnippetService._LANGUAGE, entry.name))
    return [...platformIds, ...languageIds]
  }

  /**
   * @param {string[]} tabIds
   * @returns {string | null}
   */
  _roleFor(tabIds) {
    return tabIds.length > 0 ? SnippetService._TAB_PANEL : null
  }

  /**
   * @param {string[]} tabIds
   * @returns {string | null}
   */
  _labelsFor(tabIds) {
    return tabIds.length > 0 ? tabIds.join(' ') : null
  }

  /**
   * @param {HTMLElement} tab
   * @returns {HTMLElement[]}
   */
  _tabsBeside(tab) {
    const list = tab.closest('[role="tablist"]')
    if (!list) return []
    return [...list.querySelectorAll('[role="tab"]')].map((element) => /** @type {HTMLElement} */ (element))
  }

  /**
   * @param {HTMLElement[]} tabs
   * @param {number} index
   * @param {((index: number, count: number) => number) | undefined} move
   * @returns {HTMLElement | undefined}
   */
  _nextTab(tabs, index, move) {
    if (!move) return undefined
    return tabs[move(index, tabs.length)]
  }

  /**
   * @param {KeyboardEvent} event
   * @param {HTMLElement | undefined} tab
   */
  _choose(event, tab) {
    if (!tab) return
    event.preventDefault()
    this._focusing = tab.id
    tab.focus()
    tab.click()
  }

  async _focusAfterRender() {
    const id = this._focusing
    if (!id) return
    this._focusing = ''
    await this.taskScheduler.render()
    document.getElementById(id)?.focus()
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
    return this.axis !== SnippetService._LANGUAGE && PlatformUtils.spansPlatforms(files)
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
