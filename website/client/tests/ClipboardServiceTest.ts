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

  async testABlockedClipboardSuggestsTheShortcut() {
    const clipboard = new ClipboardService()
    this.mounted(clipboard)
    this.clipboard(async () => { throw Error('denied') })
    await clipboard.copy(null, '01', 'ty preview')
    clipboard.dispose()
    Assert.equals('Press ⌘C', clipboard.label('01'))
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
