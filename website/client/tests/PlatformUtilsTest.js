// @ts-nocheck

@Test(SiteTestSetup, {"partition":"tests"})
export default class PlatformUtilsTest {
  files() {
    return [
      { name: 'client/services/BasketService.swift', code: 'final class BasketService {}' },
      { name: 'client/services/BasketService.js', code: 'export default class BasketService {}' },
    ]
  }

  async testPlatformChoicesFallBackToTheWeb() {
    Assert.equals('web', PlatformUtils.list()[0].id, 'the web is the first choice')
    Assert.equals('ios', PlatformUtils.knownPlatform('ios', 'web'), 'a known choice stays')
    Assert.equals('web', PlatformUtils.knownPlatform('nowhere', 'web'), 'an unknown choice falls back')
    Assert.equals('web', PlatformUtils.detectPlatform('curl/8.0'), 'an unknown agent is the web')
  }

  async testFilesReadInLanguageOrder() {
    Assert.equals(0, PlatformUtils.rank('client/services/BasketService.js'), 'JavaScript reads first')
    Assert.equals('client/services/BasketService.js', PlatformUtils.inReadingOrder(this.files())[0].name)
    Assert.isTrue(PlatformUtils.isCompanion('client/services/BasketService.kt'), 'Kotlin is a companion')
    Assert.isTrue(PlatformUtils.languageOf('notes.txt') === null, 'a text file has no language')
    Assert.equals('', PlatformUtils.languageLabel('notes.txt'), 'and no label')
  }

  async testReachDecidesTheFilesOfAPlatform() {
    Assert.isTrue(PlatformUtils.reachOf('server/services/OrderService.js') === null, 'server code has no reach')
    Assert.isTrue(PlatformUtils.reachOf('client/services/BasketService.js').includes('web'), 'JavaScript reaches the web')
    Assert.isTrue(PlatformUtils.spansPlatforms(this.files()), 'Swift and JavaScript reach different platforms')
    const names = PlatformUtils.filesFor(this.files(), 'web').map((file) => file.name)
    Assert.isTrue(names.includes('client/services/BasketService.js'), names.join())
  }
}
