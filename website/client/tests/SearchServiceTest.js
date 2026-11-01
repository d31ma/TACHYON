/** @import { DocumentationCatalogue } from '/models/DocumentationCatalogue.ts' */
/** @import { FeatureCatalogue } from '/models/FeatureCatalogue.ts' */
@Test(SiteTestSetup, {"partition":"tests"})
export default class SearchServiceTest {
  /**
   * @param {SearchService} service
   * @returns {{ lifetime: AbortController, renders: () => number }}
   */
  mounted(service) {
    let renders = 0
    service.taskScheduler = { render: async () => { renders += 1 } }
    const lifetime = new AbortController()
    service.lifecycle = lifetime.signal
    return { lifetime, renders: () => renders }
  }

  /** @returns {Promise<void>} */
  async pause() {
    await new Promise((resolve) => setTimeout(resolve, 0))
  }

  /** @returns {Promise<void>} */
  async searchPause() {
    await new Promise((resolve) => setTimeout(resolve, 200))
  }

  /** @returns {DocumentationCatalogue} */
  documents() {
    return {
      order: ['routing'],
      topics: { routing: { title: 'Routing', summary: 'Each folder under pages is a route.', sections: [] } },
    }
  }

  /** @returns {FeatureCatalogue} */
  features() {
    return {
      groups: ['Pages'],
      features: [{ id: 'file-routing', title: 'File-based routing', summary: 'Folders become paths.', group: 'Pages' }],
    }
  }

  /**
   * @param {SearchService} search
   * @returns {{ input: object, close: object, link: object, outside: object, dialog: object, focused: () => object | null, modal: () => boolean }}
   */
  dialogWithFocusStops(search) {
    /** @type {object | null} */
    let focused = null
    const stop = () => ({ isConnected: true, focus() { focused = this } })
    const [input, close, link, outside] = [stop(), stop(), stop(), stop()]
    let modal = false
    const dialog = {
      ...stop(),
      querySelectorAll: () => [input, close, link],
      contains: (/** @type {unknown} */ element) => [dialog, input, close, link].includes(element),
      showModal: () => { modal = true },
    }
    search._dialog = () => /** @type {HTMLDialogElement} */ (/** @type {unknown} */ (dialog))
    search._focused = () => /** @type {Element | null} */ (focused)
    return { input, close, link, outside, dialog, focused: () => focused, modal: () => modal }
  }

  /**
   * @param {SearchService} search
   * @param {boolean} backwards
   * @returns {Event}
   */
  tab(search, backwards) {
    const event = new Event('keydown', { cancelable: true })
    search.handleKey(event, 'Tab', backwards)
    return event
  }

  async testOpenCloseAndEscapeDriveShowing() {
    const search = new SearchService()
    this.mounted(search)
    search.open()
    Assert.isTrue(Boolean(search.showing), 'open shows the dialog')
    search.term = 'stale'
    search.handleKey(new Event('keydown', { cancelable: true }), 'Enter')
    Assert.isTrue(Boolean(search.showing), 'only Escape closes')
    const escape = new Event('keydown', { cancelable: true })
    search.handleKey(escape, 'Escape')
    Assert.isTrue(!search.showing && search.term === '', 'Escape closes and clears')
    Assert.isTrue(escape.defaultPrevented, 'the dialog uses Escape')
    search.open()
    search.close()
    search.dispose()
    Assert.isTrue(!search.showing && search.results.length === 0, 'close hides and forgets')
  }

  async testStatusDescribesTheQuery() {
    const search = new SearchService()
    Assert.equals('Search titles and body text across every guide.', search.status(), 'idle prompt')
    search.term = 'r'
    Assert.equals('Keep typing…', search.status(), 'one letter is not a search')
    search.term = 'zzzz'
    Assert.equals('Nothing matches “zzzz”.', search.status())
    search.results = [{ title: 'Routing', path: '/docs/routing', kind: 'guide' }]
    Assert.equals('1 result', search.status())
  }

  async testFindSearchesTheDocsAfterAPause() {
    const search = new SearchService()
    const { renders } = this.mounted(search)
    search.prepare(new Event('focus'), this.documents(), this.features())
    search.find(new Event('input'), 'routing')
    Assert.equals('Searching…', search.status(), 'a two-letter term starts a search')
    await this.searchPause()
    search.dispose()
    Assert.equals('2 results', search.status(), 'routing is in the guides and the features')
    Assert.equals(1, renders(), 'one render for one search')
  }

  async testASearchTypedBeforeTheCataloguesArriveWaitsForThem() {
    const search = new SearchService()
    const { renders } = this.mounted(search)
    search.find(new Event('input'), 'routing')
    await this.searchPause()
    Assert.isTrue(search.results.length === 0 && renders() === 0, 'nothing is searched without the catalogues')
    Assert.equals('Searching…', search.status(), 'the search waits for the catalogues')
    search.prepare(new Event('focus'), this.documents(), this.features())
    await this.searchPause()
    search.dispose()
    Assert.equals('/docs/routing', search.results[0]?.path, 'the waiting search runs once the catalogues arrive')
    Assert.equals(1, renders(), 'one render once the results are ready')
  }

  async testPreparingWithoutATermSearchesNothing() {
    const search = new SearchService()
    const { renders } = this.mounted(search)
    search.prepare(new Event('focus'), this.documents(), this.features())
    await this.searchPause()
    search.dispose()
    Assert.isTrue(search.results.length === 0 && renders() === 0, 'the catalogues alone start no search')
  }

  async testTabStaysInsideTheDialog() {
    const search = new SearchService()
    this.mounted(search)
    const page = this.dialogWithFocusStops(search)
    page.link.focus()
    Assert.isTrue(this.tab(search, false).defaultPrevented && page.focused() === page.input, 'Tab on the last stop goes to the first')
    Assert.isTrue(this.tab(search, true).defaultPrevented && page.focused() === page.link, 'Shift+Tab on the first stop goes to the last')
    page.close.focus()
    Assert.isTrue(!this.tab(search, false).defaultPrevented && page.focused() === page.close, 'Tab between stops is left to the browser')
    page.dialog.focus()
    Assert.isTrue(this.tab(search, true).defaultPrevented && page.focused() === page.link, 'Shift+Tab from the dialog itself stays inside')
    page.outside.focus()
    Assert.isTrue(this.tab(search, false).defaultPrevented && page.focused() === page.input, 'focus outside comes back in')
    const typing = new Event('keydown', { cancelable: true })
    search.handleKey(typing, 'a')
    Assert.isTrue(!typing.defaultPrevented && page.focused() === page.input, 'other keys pass through')
  }

  async testOpenShowsAModalDialog() {
    const search = new SearchService()
    this.mounted(search)
    const page = this.dialogWithFocusStops(search)
    search.open()
    await this.pause()
    search.dispose()
    Assert.isTrue(page.modal(), 'the dialog opens as a modal, above the page')
  }

  async testBackdropAndCancelClose() {
    const search = new SearchService()
    this.mounted(search)
    const page = this.dialogWithFocusStops(search)
    search.open()
    search.closeFromBackdrop(new Event('click'), /** @type {EventTarget} */ (page.link))
    Assert.isTrue(Boolean(search.showing), 'a click inside the panel keeps it open')
    search.closeFromBackdrop(new Event('click'), /** @type {EventTarget} */ (page.dialog))
    Assert.isTrue(!search.showing, 'a click on the backdrop closes')
    search.open()
    const cancel = new Event('cancel', { cancelable: true })
    search.cancel(cancel)
    search.dispose()
    Assert.isTrue(!search.showing && cancel.defaultPrevented, 'the browser cancel (Escape, Back) closes through the service')
  }

  async testCloseReturnsFocusToTheOpener() {
    const search = new SearchService()
    this.mounted(search)
    const page = this.dialogWithFocusStops(search)
    page.outside.focus()
    search.open()
    search.close()
    await this.pause()
    await this.pause()
    search.dispose()
    Assert.isTrue(page.focused() === page.outside, 'close gives focus back to what opened search')
  }

  async testHintFollowsThePlatform() {
    const search = new SearchService()
    Assert.equals('⌘K', search._hintFor('MacIntel'), 'macOS')
    Assert.equals('⌘K', search._hintFor('macOS'), 'macOS from client hints')
    Assert.equals('⌘K', search._hintFor('iPhone'), 'iOS')
    Assert.equals('Ctrl K', search._hintFor('Win32'), 'Windows')
    Assert.equals('Ctrl K', search._hintFor('Linux x86_64'), 'Linux')
  }

  async testInitializeOpensOnCommandKUntilUnmounted() {
    const search = new SearchService()
    const { lifetime } = this.mounted(search)
    search.initialize()
    Assert.isTrue(['⌘K', 'Ctrl K'].includes(search.keyHint), 'start picks the hint for this platform')
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    Assert.isTrue(Boolean(search.showing), '⌘K opens search')
    lifetime.abort()
    search.close()
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    search.dispose()
    Assert.isTrue(!search.showing, 'the listener dies with the component')
  }

  async testControlKTogglesAndOtherKeysDoNothing() {
    const search = new SearchService()
    const { lifetime } = this.mounted(search)
    search.initialize()
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k' }))
    dispatchEvent(new KeyboardEvent('keydown', { key: 'j', ctrlKey: true }))
    Assert.isTrue(!search.showing, 'only a modifier with K opens search')
    dispatchEvent(new KeyboardEvent('keydown', { key: 'K', ctrlKey: true }))
    Assert.isTrue(Boolean(search.showing), 'Ctrl+K opens search')
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', ctrlKey: true }))
    Assert.isTrue(!search.showing, 'Ctrl+K again closes search')
    lifetime.abort()
    search.dispose()
  }
}
