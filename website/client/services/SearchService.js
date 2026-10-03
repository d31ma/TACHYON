import { DocumentationUtils } from '/shared/utils/DocumentationUtils.js'

/** @import { SearchResult } from '/models/SearchResult.ts' */
/** @import { TaskScheduler } from '/models/TaskScheduler.ts' */
@Service
export default class SearchService {
  /** @type {Record<string, 'open' | 'close'>} */
  static TOGGLE = { true: 'close', false: 'open' }
  static _SHORTEST_TERM = 2

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
    setTimeout(() => this.#focusSearchInput(), 0)
  }

  close() {
    clearTimeout(this._timer)
    this.showing = false
    this.results = []
    this.term = ''
    this._searching = false
    setTimeout(() => this.#afterClose(), 0)
  }

  /**
   * @param {Event} _event
   * @param {string} key
   */
  dismiss(_event, key) {
    this.#closeOnEscape(key)
  }

  /** @returns {string} */
  status() {
    return this.#firstShown(this.#statusMessages())
  }

  /**
   * @param {Event} _event
   * @param {unknown} value
   */
  find(_event, value) {
    this.term = this.#textOf(value)
    clearTimeout(this._timer)
    this._searching = this.term.length >= SearchService._SHORTEST_TERM
    const query = this.term
    this._timer = setTimeout(() => this.#showResults(query), 120)
  }

  @OnStart
  initialize() {
    addEventListener('keydown', (event) => this.#onKeyDown(event), { signal: this.lifecycle })
  }

  @OnStop
  dispose() {
    clearTimeout(this._timer)
  }

  #focusSearchInput() {
    if (this.showing) document.getElementById('site-search-input')?.focus()
  }

  async #afterClose() {
    await this.#renderWhileMounted()
    this.#focusTriggerWhenClosed()
  }

  async #renderWhileMounted() {
    if (this.lifecycle.aborted) return
    await this.taskScheduler.render()
  }

  #focusTriggerWhenClosed() {
    if (!this.showing) /** @type {HTMLElement | null} */ (document.querySelector('.site-search__trigger button'))?.focus()
  }

  /** @param {string} key */
  #closeOnEscape(key) {
    if (key !== 'Escape') return
    this.close()
    void this.taskScheduler.render()
  }

  /** @returns {[boolean, string][]} */
  #statusMessages() {
    return [
      [this._searching, 'Searching…'],
      [this.#isTooShort(), 'Keep typing…'],
      [this.#foundNothing(), `Nothing matches “${this.term}”.`],
      [this.results.length === 1, '1 result'],
      [this.results.length > 1, `${this.results.length} results`],
      [true, 'Search titles and body text across every guide.'],
    ]
  }

  /** @returns {boolean} */
  #isTooShort() {
    return this.term.length === 1
  }

  /** @returns {boolean} */
  #foundNothing() {
    return this.term.length >= SearchService._SHORTEST_TERM && this.results.length === 0
  }

  /**
   * @param {[boolean, string][]} messages
   * @returns {string}
   */
  #firstShown(messages) {
    return messages.find(([shown]) => shown)?.[1] ?? ''
  }

  /**
   * @param {unknown} value
   * @returns {string}
   */
  #textOf(value) {
    return String(value ?? '')
  }

  /** @param {string} query */
  async #showResults(query) {
    if (this.lifecycle.aborted) return
    this.results = DocumentationUtils.searchDocs(query)
    this._searching = false
    await this.taskScheduler.render()
  }

  /** @param {Event} event */
  #onKeyDown(event) {
    const keyboard = /** @type {KeyboardEvent} */ (event)
    const pressed = [this.#hasModifier(keyboard), keyboard.key.toLowerCase() === 'k']
    if (pressed.includes(false)) return
    keyboard.preventDefault()
    const action = SearchService.TOGGLE[String(this.showing)]
    this[action]()
    void this.taskScheduler.render()
  }

  /**
   * @param {KeyboardEvent} event
   * @returns {boolean}
   */
  #hasModifier(event) {
    return [event.metaKey, event.ctrlKey].includes(true)
  }
}
