// @ts-nocheck

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

  snippet(files, axis, saved = null) {
    const snippet = new SnippetService(files, axis)
    snippet.preferenceStrings = { read: (key) => (key === 'tachyon.platform' ? saved : null) }
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
    snippet.pick(null, 'Python')
    const shown = snippet.shown().map((file) => file.language)
    Assert.isTrue(shown.includes('Python') && !shown.includes('Swift'), shown.join())
  }

  async testPlatformChangedAppliesAndResetsTheLanguage() {
    const snippet = this.snippet(this.files())
    snippet.companion = 'Swift'
    snippet.platformChanged({ platform: 'ios' })
    const current = snippet.platforms().find((platform) => platform.current)
    Assert.isTrue(current.id === 'ios' && snippet.companion === '', 'a new platform resets the language')
  }

  async testTheSavedPlatformIsTheFirstChoice() {
    const snippet = this.snippet(this.files(), undefined, 'ios')
    const current = snippet.platforms().find((platform) => platform.current)
    Assert.equals('ios', current.id, 'the platform saved in preferences is chosen')
    const unknown = this.snippet(this.files(), undefined, 'amiga')
    Assert.isTrue(unknown.platforms().some((platform) => platform.current), 'an unknown saved platform falls back to a known one')
  }
}
