globalThis.__tachyonTitleBar ??= new (class TacTitleBar {
  static LIMIT = 256
  static CONTROLS = [
    'a[href]',
    'button',
    'input',
    'select',
    'textarea',
    'summary',
    'label',
    'iframe',
    'video[controls]',
    'audio[controls]',
    '[contenteditable]:not([contenteditable="false"])',
    '[tabindex]:not([tabindex="-1"])',
    '[role="button"]',
    '[role="link"]',
    '[role="tab"]',
    '[role="checkbox"]',
    '[role="radio"]',
    '[role="switch"]',
    '[role="menuitem"]',
    '[role="slider"]',
    '[role="textbox"]',
    '[role="combobox"]',
    '[data-tac-no-drag]',
  ].join(', ')

  constructor() {
    this._sheet = new CSSStyleSheet()
    this._sent = ''
    this._scheduled = false
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, this._sheet]
    addEventListener('DOMContentLoaded', () => this.watch(), { once: true })
  }

  /** @param {{ height: number, leading: number, trailing: number }} insets */
  apply(insets) {
    const height = `--tac-titlebar-height: ${insets.height}px;`
    const leading = `--tac-titlebar-leading-inset: ${insets.leading}px;`
    const trailing = `--tac-titlebar-trailing-inset: ${insets.trailing}px;`
    this._sheet.replaceSync(`:root { ${height} ${leading} ${trailing} }`)
    this.schedule()
  }

  watch() {
    const schedule = () => this.schedule()
    addEventListener('resize', schedule)
    addEventListener('scroll', schedule, true)
    addEventListener('load', schedule, true)
    const changes = { attributes: true, childList: true, characterData: true, subtree: true }
    new MutationObserver(schedule).observe(document.documentElement, changes)
    this.schedule()
  }

  schedule() {
    if (this._scheduled) return
    this._scheduled = true
    requestAnimationFrame(() => {
      this._scheduled = false
      this.report()
    })
  }

  report() {
    /** @type {{ drag: number[][], exclude: number[][] }} */
    const regions = { drag: [], exclude: [] }
    for (const region of document.querySelectorAll('[data-tac-drag-region]')) {
      TacTitleBar.add(regions.drag, region)
      for (const control of region.querySelectorAll(TacTitleBar.CONTROLS)) {
        TacTitleBar.add(regions.exclude, control)
      }
    }
    const tooMany = regions.drag.length > TacTitleBar.LIMIT || regions.exclude.length > TacTitleBar.LIMIT
    const payload = JSON.stringify(tooMany ? { drag: [], exclude: [] } : regions)
    if (payload === this._sent) return
    this._sent = payload
    globalThis.__tachyonNativeHostCall?.('window.dragRegions', payload)
  }

  /**
   * @param {number[][]} list
   * @param {Element} element
   */
  static add(list, element) {
    const box = element.getBoundingClientRect()
    if (box.width <= 0 || box.height <= 0) return
    list.push([Math.round(box.left), Math.round(box.top), Math.round(box.width), Math.round(box.height)])
  }
})()
