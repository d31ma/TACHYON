import { DocumentationUtils } from '/shared/utils/DocumentationUtils.js'

/** @import { DocumentationCatalogue } from '/models/DocumentationCatalogue.ts' */
/** @import { FeatureCatalogue } from '/models/FeatureCatalogue.ts' */
/** @import { SearchResult } from '/models/SearchResult.ts' */
/** @import { TaskScheduler } from '/models/TaskScheduler.ts' */
@Service
export default class SearchService {
  /** @type {TaskScheduler} */
  @Needs
  taskScheduler
  /** @type {AbortSignal} */
  @Needs
  lifecycle
  showing = false
  /** @type {SearchResult[]} */
  results = []
  term = ''
  keyHint = 'Ctrl K'
  _searching = false
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  _timer = undefined
  /** @type {HTMLElement | null} */
  _opener = null
  /** @type {{ documents: DocumentationCatalogue, features: FeatureCatalogue } | undefined} */
  _catalogues = undefined

  open() {
    this._opener = this._openerOf(this._focused())
    this.showing = true
    setTimeout(() => this._presentDialog(), 0)
  }

  close() {
    clearTimeout(this._timer)
    this.showing = false
    this.results = []
    this.term = ''
    this._searching = false
    setTimeout(() => this._afterClose(), 0)
  }

  /** @param {Event} event */
  cancel(event) {
    event.preventDefault()
    this.close()
  }

  /**
   * @param {Event} _event
   * @param {EventTarget | null} target
   */
  closeFromBackdrop(_event, target) {
    this._closeWhen(target === this._dialog())
  }

  /**
   * @param {Event} event
   * @param {string} key
   * @param {boolean} backwards
   */
  handleKey(event, key, backwards = false) {
    this._closeOnEscape(event, key)
    const dialog = this._dialog()
    const focused = this._focused()
    const order = this._inTabOrder(this._focusStops(dialog), backwards)
    const leaving = this._leavesDialog(this._isInside(dialog, focused), focused, order.at(-1))
    this._wrapFocusOnTab(event, key, leaving, order[0])
  }

  /** @returns {string} */
  status() {
    return this._statusMessage()
  }

  /**
   * @param {Event} _event
   * @param {unknown} value
   */
  find(_event, value) {
    this.term = this._textOf(value)
    clearTimeout(this._timer)
    this._searching = this.term.length >= DocumentationUtils.shortestTerm()
    const query = this.term
    this._timer = setTimeout(() => this._showResults(query), 120)
  }

  /**
   * @param {Event} event
   * @param {DocumentationCatalogue} documents
   * @param {FeatureCatalogue} features
   */
  prepare(event, documents, features) {
    this._catalogues = { documents, features }
    this._searchAgainWhen(event, this._searching)
  }

  @OnStart
  initialize() {
    this.keyHint = this._hintFor(this._platformName())
    addEventListener('keydown', (event) => this._onKeyDown(event, this._isShortcut(event)), { signal: this.lifecycle })
  }

  @OnStop
  dispose() {
    clearTimeout(this._timer)
  }

  /** @returns {Element | null} */
  _focused() {
    return document.activeElement
  }

  /** @returns {HTMLDialogElement | null} */
  _dialog() {
    return /** @type {HTMLDialogElement | null} */ (document.getElementById('site-search-dialog'))
  }

  /** @returns {HTMLElement | null} */
  _trigger() {
    return /** @type {HTMLElement | null} */ (document.querySelector('.site-search__trigger button'))
  }

  /**
   * @param {Element | null} focused
   * @returns {HTMLElement | null}
   */
  _openerOf(focused) {
    return focused === document.body ? null : /** @type {HTMLElement | null} */ (focused)
  }

  _presentDialog() {
    if (!this.showing) return
    this._dialog()?.showModal()
    document.getElementById('site-search-input')?.focus()
  }

  /** @param {boolean} outside */
  _closeWhen(outside) {
    if (!outside) return
    this.close()
    void this.taskScheduler.render()
  }

  async _afterClose() {
    await this._renderWhileMounted()
    const target = this._returnTarget(this._connectedOpener(), this._trigger())
    this._opener = null
    this._focusWhenClosed(target)
  }

  async _renderWhileMounted() {
    if (this.lifecycle.aborted) return
    await this.taskScheduler.render()
  }

  /** @returns {HTMLElement | null} */
  _connectedOpener() {
    return this._opener?.isConnected ? this._opener : null
  }

  /**
   * @param {HTMLElement | null} opener
   * @param {HTMLElement | null} trigger
   * @returns {HTMLElement | null}
   */
  _returnTarget(opener, trigger) {
    return opener ?? trigger
  }

  /** @param {HTMLElement | null} target */
  _focusWhenClosed(target) {
    if (!this.showing) target?.focus()
  }

  /**
   * @param {Event} event
   * @param {string} key
   */
  _closeOnEscape(event, key) {
    if (key !== 'Escape') return
    event.preventDefault()
    this.close()
    void this.taskScheduler.render()
  }

  /**
   * @param {HTMLElement | null} dialog
   * @returns {HTMLElement[]}
   */
  _focusStops(dialog) {
    return /** @type {HTMLElement[]} */ ([...(dialog?.querySelectorAll('input, button, a[href]') ?? [])])
  }

  /**
   * @param {HTMLElement[]} stops
   * @param {boolean} backwards
   * @returns {HTMLElement[]}
   */
  _inTabOrder(stops, backwards) {
    return backwards ? stops.toReversed() : stops
  }

  /**
   * @param {HTMLElement | null} dialog
   * @param {Element | null} focused
   * @returns {boolean}
   */
  _isInside(dialog, focused) {
    return Boolean(dialog?.contains(focused)) && focused !== dialog
  }

  /**
   * @param {boolean} inside
   * @param {Element | null} focused
   * @param {HTMLElement | undefined} last
   * @returns {boolean}
   */
  _leavesDialog(inside, focused, last) {
    return !inside || focused === last
  }

  /**
   * @param {Event} event
   * @param {string} key
   * @param {boolean} leaving
   * @param {HTMLElement | undefined} first
   */
  _wrapFocusOnTab(event, key, leaving, first) {
    if (key !== 'Tab' || !leaving) return
    event.preventDefault()
    first?.focus()
  }

  /** @returns {string} */
  _statusMessage() {
    let message = 'Search titles and body text across every guide.'
    if (this._searching) {
      message = 'Searching…'
    } else if (this.term.length === 1) {
      message = 'Keep typing…'
    } else if (this.term.length >= DocumentationUtils.shortestTerm() && this.results.length === 0) {
      message = `Nothing matches “${this.term}”.`
    } else if (this.results.length === 1) {
      message = '1 result'
    } else if (this.results.length > 1) {
      message = `${this.results.length} results`
    }
    return message
  }

  /**
   * @param {unknown} value
   * @returns {string}
   */
  _textOf(value) {
    return String(value ?? '')
  }

  /**
   * @param {Event} event
   * @param {boolean} waiting
   */
  _searchAgainWhen(event, waiting) {
    if (!waiting) return
    this.find(event, this.term)
  }

  /** @param {string} query */
  async _showResults(query) {
    if (this.lifecycle.aborted || !this._catalogues) return
    this.results = DocumentationUtils.search(query, this._catalogues.documents, this._catalogues.features)
    this._searching = false
    await this.taskScheduler.render()
  }

  /** @returns {string} */
  _platformName() {
    const browser = /** @type {Navigator & { userAgentData?: { platform: string } }} */ (navigator)
    return browser.userAgentData?.platform ?? browser.platform
  }

  /**
   * @param {string} platform
   * @returns {string}
   */
  _hintFor(platform) {
    return /mac|iphone|ipad|ipod/i.test(platform) ? '⌘K' : 'Ctrl K'
  }

  /**
   * @param {Event} event
   * @returns {boolean}
   */
  _isShortcut(event) {
    const keyboard = /** @type {KeyboardEvent} */ (event)
    if (keyboard.metaKey || keyboard.ctrlKey) return keyboard.key.toLowerCase() === 'k'
    return false
  }

  /**
   * @param {Event} event
   * @param {boolean} shortcut
   */
  _onKeyDown(event, shortcut) {
    if (shortcut && this.showing) {
      this.close()
    } else if (shortcut) {
      this.open()
    } else {
      return
    }
    event.preventDefault()
    void this.taskScheduler.render()
  }
}
