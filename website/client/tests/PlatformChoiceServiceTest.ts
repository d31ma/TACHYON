@Test(SiteTestSetup, {"partition":"tests"})
export default class PlatformChoiceServiceTest {
  async testChoosePublishesAndRemembersThePlatform() {
    const choice = new PlatformChoiceService()
    const published: unknown[][] = []
    choice.applicationEventPublisher = { publishEvent: (...event: unknown[]) => { published.push(event) } }
    choice.choose(null, 'windows')
    const [event, value, retain] = published[0] ?? []
    if (event !== 'PlatformChanged' || (value as { platform: string }).platform !== 'windows') throw Error('choose announces the PlatformChanged model')
    if (retain !== true) throw Error('retained, so a snippet mounted later reads it')
    if (localStorage.getItem('tachyon.platform') !== 'windows') throw Error('and remembers it')
  }
}
