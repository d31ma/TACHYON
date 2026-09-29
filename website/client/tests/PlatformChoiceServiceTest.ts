@Test(SiteTestSetup, {"partition":"tests"})
export default class PlatformChoiceServiceTest {
  async testChoosePublishesAndRemembersThePlatform() {
    const choice = new PlatformChoiceService()
    const published: unknown[][] = []
    choice.applicationEventPublisher = { publishEvent: (...event: unknown[]) => { published.push(event) } }
    choice.choose(null, 'windows')
    const [event, value, retain] = published[0] ?? []
    Assert.equals('PlatformChanged', event, 'choose announces the PlatformChanged model')
    Assert.equals('windows', (value as { platform: string }).platform, 'choose announces the platform')
    Assert.equals(true, retain, 'retained, so a snippet mounted later reads it')
    Assert.equals('windows', localStorage.getItem('tachyon.platform'), 'and remembers it')
  }
}
