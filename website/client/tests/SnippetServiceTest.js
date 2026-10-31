@Test(SiteTestSetup, {"partition":"tests"})
export default class SnippetServiceTest {
  files() {
    return [
      { name: 'client/services/BasketService.swift', code: 'final class BasketService {}' },
      { name: 'client/services/BasketService.js', code: 'export default class BasketService {}' },
      { name: 'client/services/BasketService.py', code: 'class BasketService:\n    pass' },
      { name: 'client/pages/tac.html', code: '<basket-tally />' },
    ]
  }

  /**
   * @param {unknown} files
   * @param {string} [axis]
   * @param {string | null} [saved]
   * @returns {SnippetService}
   */
  snippet(files, axis = '', saved = null) {
    const snippet = new SnippetService()
    snippet.files = /** @type {SnippetService['files']} */ (files)
    snippet.axis = axis
    snippet.preferenceStrings = { read: (key) => (key === 'tachyon.platform' ? saved : null), write: () => true }
    return snippet
  }

  async testListAndReadingOrder() {
    const snippet = this.snippet(this.files())
    Assert.equals(4, snippet.files.length, 'files are the ones it was given')
    Assert.isTrue(!this.snippet('nope').spans(), 'anything else is no files')
    snippet.platformChanged({ platform: 'web' })
    Assert.equals('client/services/BasketService.js', snippet.shown()[0].name, 'JavaScript reads first')
  }

  async testSpansAndPlatformsFollowReach() {
    const snippet = this.snippet(this.files())
    Assert.isTrue(Boolean(snippet.spans()), 'Python reaches only the web, so the files reach different platforms')
    Assert.isTrue(!this.snippet(this.files(), 'language').spans(), 'a language axis never spans')
    snippet.platformChanged({ platform: 'android' })
    const current = snippet.platforms().filter((platform) => platform.current).map((platform) => platform.id)
    Assert.equals(['android'], current, 'one current platform')
  }

  async testCompanionsAndPickChooseALanguage() {
    const snippet = this.snippet(this.files(), 'language')
    const names = snippet.companions().map((entry) => entry.name)
    Assert.equals('JavaScript,Python,Swift', names.join())
    snippet.pick(new Event('click'), 'Python')
    const shown = snippet.shown().map((file) => file.language)
    Assert.isTrue(shown.includes('Python') && !shown.includes('Swift'), shown.join())
  }

  async testPlatformChangedAppliesAndResetsTheLanguage() {
    const snippet = this.snippet(this.files())
    snippet.companion = 'Swift'
    snippet.platformChanged({ platform: 'ios' })
    const current = snippet.platforms().find((platform) => platform.current)
    Assert.isTrue(current?.id === 'ios' && snippet.companion === '', 'a new platform resets the language')
  }

  async testTheSavedPlatformIsTheFirstChoice() {
    const snippet = this.snippet(this.files(), undefined, 'ios')
    const current = snippet.platforms().find((platform) => platform.current)
    Assert.equals('ios', current?.id, 'the platform saved in preferences is chosen')
    const unknown = this.snippet(this.files(), undefined, 'amiga')
    Assert.isTrue(unknown.platforms().some((platform) => platform.current), 'an unknown saved platform falls back to a known one')
  }

  /**
   * @param {string[]} names
   * @param {string[]} moves
   * @returns {HTMLElement[]}
   */
  tabs(names, moves) {
    const list = { querySelectorAll: () => tabs }
    const tabs = names.map((name) => ({
      id: name,
      closest: () => list,
      focus: () => moves.push(`focus ${name}`),
      click: () => moves.push(`click ${name}`),
    }))
    return /** @type {HTMLElement[]} */ (/** @type {unknown} */ (tabs))
  }

  /**
   * @param {SnippetService} snippet
   * @param {HTMLElement} tab
   * @param {string} key
   * @returns {boolean}
   */
  press(snippet, tab, key) {
    let prevented = false
    const event = { target: tab, preventDefault: () => { prevented = true } }
    snippet.moveTab(/** @type {KeyboardEvent} */ (/** @type {unknown} */ (event)), key)
    return prevented
  }

  async testArrowsHomeAndEndMoveAndChooseTheTab() {
    const snippet = this.snippet(this.files(), 'language')
    /** @type {string[]} */
    const moves = []
    const tabs = this.tabs(['JavaScript', 'Python', 'Swift'], moves)
    Assert.isTrue(this.press(snippet, tabs[0], 'ArrowRight'), 'an arrow key is handled')
    this.press(snippet, tabs[2], 'ArrowRight')
    this.press(snippet, tabs[0], 'ArrowLeft')
    this.press(snippet, tabs[2], 'Home')
    this.press(snippet, tabs[0], 'End')
    Assert.equals('focus Python,click Python,focus JavaScript,click JavaScript,focus Swift,click Swift,focus JavaScript,click JavaScript,focus Swift,click Swift', moves.join(), 'arrows wrap around, Home and End go to the ends')
    Assert.isTrue(!this.press(snippet, tabs[1], 'Enter') && moves.length === 10, 'other keys are left alone')
  }

  async testTheChosenTabKeepsFocusAfterRendering() {
    const snippet = this.snippet(this.files(), 'language')
    let renders = 0
    snippet.taskScheduler = { render: async () => { renders += 1 } }
    /** @type {string[]} */
    const moves = []
    const tabs = this.tabs(['JavaScript', 'Python'], moves)
    snippet.pick(new Event('click'), 'Python')
    Assert.equals(0, renders, 'a click needs no extra render')
    this.press(snippet, tabs[0], 'ArrowRight')
    snippet.pick(new Event('click'), 'Python')
    await Promise.resolve()
    Assert.equals(1, renders, 'a key move renders once, then focuses the chosen tab')
    Assert.equals('', snippet._focusing)
  }

  async testTabsAndPanelAreLinked() {
    const snippet = this.snippet(this.files(), 'language')
    snippet.pick(new Event('click'), 'Python')
    const tab = snippet.languageTabId('Python')
    Assert.isTrue(tab.startsWith(`${snippet.panelId()}-language-`), tab)
    Assert.equals(tab, snippet.panelLabels(), 'the panel is labelled by the selected tab')
    Assert.equals('tabpanel', snippet.panelRole())
    Assert.isTrue(snippet.languageTabId('C#') !== snippet.languageTabId('C++'), 'names that differ only in symbols get their own ids')
    Assert.isTrue(/^[\w-]+$/.test(snippet.languageTabId('C++')), 'an id holds only letters, digits and dashes')
    Assert.isTrue(this.snippet(this.files()).panelId() !== snippet.panelId(), 'each snippet has its own panel')
  }

  async testThePlatformTabLabelsThePanel() {
    const snippet = this.snippet(this.files())
    snippet.platformChanged({ platform: 'web' })
    const labels = String(snippet.panelLabels())
    Assert.isTrue(labels.startsWith(`${snippet.platformTabId('web')} `), labels)
  }

  async testOneFileHasNoTabPanel() {
    const snippet = this.snippet([{ name: 'client/pages/tac.html', code: '<basket-tally />' }], 'language')
    Assert.equals(null, snippet.panelRole(), 'without tabs there is no tab panel')
    Assert.equals(null, snippet.panelLabels())
  }
}
