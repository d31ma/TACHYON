// @ts-nocheck

@Test(SiteTestSetup, {"partition":"tests"})
export default class SearchServiceTest {
  clipboard(write) {
    Object.defineProperty(navigator, 'clipboard', { value: { writeText: write }, configurable: true })
  }

  mounted(service) {
    service.renders = 0
    service.taskScheduler = { render: async () => { service.renders += 1 } }
    const lifetime = new AbortController()
    service.lifecycle = lifetime.signal
    return lifetime
  }

  async testOpenCloseAndDismissDriveShowing() {
    const search = new SearchService()
    this.mounted(search)
    search.open()
    Assert.isTrue(Boolean(search.showing), 'open shows the dialog')
    search.term = 'stale'
    search.dismiss(null, 'Tab')
    Assert.isTrue(Boolean(search.showing), 'only Escape dismisses')
    search.dismiss(null, 'Escape')
    Assert.isTrue(!search.showing && search.term === '', 'Escape closes and clears')
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
    search.results = [{}]
    Assert.equals('1 result', search.status())
  }

  async testFindSearchesTheDocsAfterAPause() {
    const search = new SearchService()
    this.mounted(search)
    search.find(null, 'routing')
    Assert.equals('Searching…', search.status(), 'a two-letter term starts a search')
    await new Promise((resolve) => setTimeout(resolve, 200))
    search.dispose()
    Assert.isTrue(search.status() !== 'Searching…' && search.results.length > 0, 'routing is in the docs')
    Assert.equals(1, search.renders, 'one render for one search')
  }

  async testInitializeOpensOnCommandKUntilUnmounted() {
    const search = new SearchService()
    const lifetime = this.mounted(search)
    search.initialize()
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    Assert.isTrue(Boolean(search.showing), '⌘K opens search')
    lifetime.abort()
    search.close()
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    search.dispose()
    Assert.isTrue(!search.showing, 'the listener dies with the component')
  }
}
