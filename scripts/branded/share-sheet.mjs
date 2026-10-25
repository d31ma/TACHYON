/**
 * navigator.share() settles only when a person answers the system share sheet. A branded run has
 * no person at the screen, so the page's navigator.share() still calls the browser's own share,
 * and after a short wait the sheet is closed for the person. The page then hears the AbortError a
 * person closing the sheet gives. The host decides how the sheet closes: Chrome on Android presses
 * Back on the real sheet; headless desktop Chrome and Edge show no sheet at all, so the page gives
 * the AbortError itself.
 */
export class ShareSheet {
  static binding = '__tachyonBrandedCloseShareSheet'
  static closeMilliseconds = 2_000

  /**
   * Runs in the page before its own scripts.
   * @param {string} binding @param {number} closeMilliseconds
   */
  static pageScript = (binding, closeMilliseconds) => {
    const share = navigator.share?.bind(navigator)
    if (!share) return
    navigator.share = (data) => new Promise((resolve, reject) => {
      let settled = false
      const settle = (finish) => (value) => {
        if (settled) return
        settled = true
        finish(value)
      }
      share(data).then(settle(resolve), settle(reject))
      setTimeout(async () => {
        if (settled) return
        const closedBySystem = await window[binding]()
        if (!closedBySystem) settle(reject)(new DOMException('The share sheet was closed for the person in a branded browser run.', 'AbortError'))
      }, closeMilliseconds)
    })
  }

  /**
   * @param {import('playwright').BrowserContext | import('playwright').Page} target
   * @param {() => Promise<boolean>} close answers true when the system closed a real sheet
   */
  static async install(target, close) {
    await target.exposeBinding(ShareSheet.binding, () => close())
    await target.addInitScript({ content: `(${ShareSheet.pageScript})(${JSON.stringify(ShareSheet.binding)}, ${ShareSheet.closeMilliseconds})` })
  }

  /** @param {string} label */
  static withoutSheet(label) {
    return async () => {
      console.error(`branded: navigator.share() was still waiting after ${ShareSheet.closeMilliseconds} ms; headless ${label} shows no share sheet, so it was closed for the person with an AbortError`)
      return false
    }
  }
}
