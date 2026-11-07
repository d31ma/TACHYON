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
    Assert.equals('client/services/BasketService.js', PlatformUtils.inReadingOrder(this.files())[0].name)
    Assert.isTrue(PlatformUtils.isCompanion('client/services/BasketService.kt'), 'Kotlin is a companion')
    Assert.equals('', PlatformUtils.languageLabel('notes.txt'), 'a text file has no language')
  }

  async testReachDecidesTheFilesOfAPlatform() {
    const server = [{ name: 'server/services/OrderService.js', code: '' }]
    Assert.equals(1, PlatformUtils.filesFor(server, 'ios').length, 'server code is shown on every platform')
    Assert.isTrue(!PlatformUtils.spansPlatforms(this.files()), 'Swift and JavaScript reach the same platforms')
    const python = [...this.files(), { name: 'client/services/BasketService.py', code: '' }]
    Assert.isTrue(PlatformUtils.spansPlatforms(python), 'Python reaches only the web')
    const names = PlatformUtils.filesFor(this.files(), 'web').map((file) => file.name)
    Assert.isTrue(names.includes('client/services/BasketService.js'), names.join())
  }
}
