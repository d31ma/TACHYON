import { DocumentationUtils } from '/shared/utils/DocumentationUtils.js'

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
  _searching = false
  /** @type {ReturnType<typeof setTimeout> | undefined} */
  _timer = undefined

  open() {
    this.showing = true
    setTimeout(() => this._focusSearchInput(), 0)
  }

  close() {
    clearTimeout(this._timer)
    this.showing = false
    this.results = []
    this.term = ''
    this._searching = false
    setTimeout(() => this._afterClose(), 0)
  }

  /**
   * @param {Event} _event
   * @param {string} key
   */
  dismiss(_event, key) {
    this._closeOnEscape(key)
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

  @OnStart
  initialize() {
    addEventListener('keydown', (event) => this._onKeyDown(event), { signal: this.lifecycle })
  }

  @OnStop
  dispose() {
    clearTimeout(this._timer)
  }

  _focusSearchInput() {
    if (this.showing) document.getElementById('site-search-input')?.focus()
  }

  async _afterClose() {
    await this._renderWhileMounted()
    this._focusTriggerWhenClosed()
  }

  async _renderWhileMounted() {
    if (this.lifecycle.aborted) return
    await this.taskScheduler.render()
  }

  _focusTriggerWhenClosed() {
    if (!this.showing) /** @type {HTMLElement | null} */ (document.querySelector('.site-search__trigger button'))?.focus()
  }

  /** @param {string} key */
  _closeOnEscape(key) {
    if (key !== 'Escape') return
    this.close()
    void this.taskScheduler.render()
  }

  /** @returns {string} */
  _statusMessage() {
    if (this._searching) {
      return 'Searching…'
    } else if (this.term.length === 1) {
      return 'Keep typing…'
    } else if (this.term.length >= DocumentationUtils.shortestTerm() && this.results.length === 0) {
      return `Nothing matches “${this.term}”.`
    } else if (this.results.length === 1) {
      return '1 result'
    } else if (this.results.length > 1) {
      return `${this.results.length} results`
    }
    return 'Search titles and body text across every guide.'
  }

  /**
   * @param {unknown} value
   * @returns {string}
   */
  _textOf(value) {
    return String(value ?? '')
  }

  /** @param {string} query */
  async _showResults(query) {
    if (this.lifecycle.aborted) return
    this.results = DocumentationUtils.search(query)
    this._searching = false
    await this.taskScheduler.render()
  }

  /** @param {Event} event */
  _onKeyDown(event) {
    const keyboard = /** @type {KeyboardEvent} */ (event)
    if (keyboard.key.toLowerCase() !== 'k' || !(keyboard.metaKey || keyboard.ctrlKey)) {
      return
    } else if (this.showing) {
      this.close()
    } else {
      this.open()
    }
    keyboard.preventDefault()
    void this.taskScheduler.render()
  }
}
