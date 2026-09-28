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
    if (!search.showing) throw Error('open shows the dialog')
    search.term = 'stale'
    search.dismiss(null, 'Tab')
    if (!search.showing) throw Error('only Escape dismisses')
    search.dismiss(null, 'Escape')
    if (search.showing || search.term !== '') throw Error('Escape closes and clears')
    search.open()
    search.close()
    search.dispose()
    if (search.showing || search.results.length !== 0) throw Error('close hides and forgets')
  }

  async testStatusDescribesTheQuery() {
    const search = new SearchService()
    if (search.status() !== 'Search titles and body text across every guide.') throw Error('idle prompt')
    search.term = 'r'
    if (search.status() !== 'Keep typing…') throw Error('one letter is not a search')
    search.term = 'zzzz'
    if (search.status() !== 'Nothing matches “zzzz”.') throw Error(search.status())
    search.results = [{}]
    if (search.status() !== '1 result') throw Error(search.status())
  }

  async testFindSearchesTheDocsAfterAPause() {
    const search = new SearchService()
    this.mounted(search)
    search.find(null, 'routing')
    if (search.status() !== 'Searching…') throw Error('a two-letter term starts a search')
    await new Promise((resolve) => setTimeout(resolve, 200))
    search.dispose()
    if (search.status() === 'Searching…' || search.results.length === 0) throw Error('routing is in the docs')
    if (search.renders !== 1) throw Error('one render for one search')
  }

  async testInitializeOpensOnCommandKUntilUnmounted() {
    const search = new SearchService()
    const lifetime = this.mounted(search)
    search.initialize()
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    if (!search.showing) throw Error('⌘K opens search')
    lifetime.abort()
    search.close()
    dispatchEvent(new KeyboardEvent('keydown', { key: 'k', metaKey: true }))
    search.dispose()
    if (search.showing) throw Error('the listener dies with the component')
  }
}
