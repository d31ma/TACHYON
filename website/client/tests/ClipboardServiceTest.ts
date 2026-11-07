@Test(SiteTestSetup, {"partition":"tests"})
export default class ClipboardServiceTest {
  clipboard(write: (text: string) => Promise<void>): void {
    Object.defineProperty(navigator, 'clipboard', { value: { writeText: write }, configurable: true })
  }

  mounted(service: ClipboardService): { lifetime: AbortController, renders: () => number } {
    let renders = 0
    service.taskScheduler = { render: async () => { renders += 1 } }
    const lifetime = new AbortController()
    service.lifecycle = lifetime.signal
    return { lifetime, renders: () => renders }
  }

  async testCopyWritesTheTextAndLabelsOnlyItsButton() {
    const clipboard = new ClipboardService()
    const { renders } = this.mounted(clipboard)
    const written: string[] = []
    this.clipboard(async (text) => { written.push(text) })
    Assert.equals('Copy', clipboard.label('install'), 'a button starts as Copy')
    await clipboard.copy(null, 'install', 'ty init')
    clipboard.dispose()
    Assert.equals('ty init', written[0], 'copy writes the text it was given')
    Assert.isTrue(clipboard.label('install') === 'Copied' && clipboard.label('02') === 'Copy', 'only that button says so')
    Assert.equals(1, renders(), 'and renders once')
  }

  async blockedCopyLabel(platform: string, userAgent: string): Promise<string> {
    Object.defineProperty(navigator, 'platform', { value: platform, configurable: true })
    Object.defineProperty(navigator, 'userAgent', { value: userAgent, configurable: true })
    const clipboard = new ClipboardService()
    this.mounted(clipboard)
    this.clipboard(async () => { throw Error('denied') })
    await clipboard.copy(null, '01', 'ty preview')
    clipboard.dispose()
    return clipboard.label('01')
  }

  async testABlockedClipboardSuggestsTheMacShortcutOnApplePlatforms() {
    Assert.equals('Press ⌘C', await this.blockedCopyLabel('MacIntel', 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)'), 'a Mac')
    Assert.equals('Press ⌘C', await this.blockedCopyLabel('iPhone', 'Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X)'), 'an iPhone')
  }

  async testABlockedClipboardSuggestsCtrlCElsewhere() {
    Assert.equals('Press Ctrl+C', await this.blockedCopyLabel('Win32', 'Mozilla/5.0 (Windows NT 10.0; Win64; x64)'), 'Windows')
    Assert.equals('Press Ctrl+C', await this.blockedCopyLabel('Linux x86_64', 'Mozilla/5.0 (X11; Linux x86_64)'), 'Linux')
  }

  async testCopyDoesNothingOnceUnmounted() {
    const clipboard = new ClipboardService()
    const { lifetime, renders } = this.mounted(clipboard)
    lifetime.abort()
    let touched = false
    this.clipboard(async () => { touched = true })
    await clipboard.copy(null, 'install', 'ty init')
    clipboard.dispose()
    Assert.isTrue(!touched && renders() === 0, 'an unmounted button neither writes nor renders')
  }
}
