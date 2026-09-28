// @ts-nocheck

@Test(SiteTestSetup, {"partition":"tests"})
export default class SnippetServiceTest {
  files() {
    return [
      { name: 'client/services/BasketService.swift', code: 'final class BasketService {}' },
      { name: 'client/services/BasketService.js', code: 'export default class BasketService {}' },
      { name: 'client/services/BasketService.kt', code: 'class BasketService' },
      { name: 'client/pages/tac.html', code: '<basket-tally />' },
    ]
  }

  async testListAndReadingOrder() {
    const snippet = new SnippetService(this.files())
    if (snippet.files.length !== 4) throw Error('files are the ones it was given')
    if (new SnippetService('nope').spans()) throw Error('anything else is no files')
    snippet.platformChanged({ platform: 'web' })
    if (snippet.shown()[0].name !== 'client/services/BasketService.js') throw Error('JavaScript reads first')
  }

  async testSpansAndPlatformsFollowReach() {
    const snippet = new SnippetService(this.files())
    if (!snippet.spans()) throw Error('Swift, JavaScript and Kotlin reach different platforms')
    if (new SnippetService(this.files(), 'language').spans()) throw Error('a language axis never spans')
    snippet.platformChanged({ platform: 'android' })
    const current = snippet.platforms().filter((platform) => platform.current)
    if (current.length !== 1 || current[0].id !== 'android') throw Error('one current platform')
  }

  async testCompanionsAndPickChooseALanguage() {
    const snippet = new SnippetService(this.files(), 'language')
    const names = snippet.companions().map((entry) => entry.name)
    if (names.join() !== 'JavaScript,Swift,Kotlin') throw Error(names.join())
    snippet.pick(null, 'Kotlin')
    const shown = snippet.shown().map((file) => file.language)
    if (!shown.includes('Kotlin') || shown.includes('Swift')) throw Error(shown.join())
  }

  async testPlatformChangedAppliesAndResetsTheLanguage() {
    const snippet = new SnippetService(this.files())
    snippet.companion = 'Swift'
    snippet.platformChanged({ platform: 'ios' })
    const current = snippet.platforms().find((platform) => platform.current)
    if (current.id !== 'ios' || snippet.companion !== '') throw Error('a new platform resets the language')
  }
}
