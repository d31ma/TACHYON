globalThis.__tachyonTitleBar ??= new (class TacMobileTitleBar {
  constructor() {
    this._sheet = new CSSStyleSheet()
    document.adoptedStyleSheets = [...document.adoptedStyleSheets, this._sheet]
    queueMicrotask(() => this.request())
  }

  /** @param {{ height: number, leading: number, trailing: number }} insets */
  apply(insets) {
    const height = `--tac-titlebar-height: ${insets.height}px;`
    const leading = `--tac-titlebar-leading-inset: ${insets.leading}px;`
    const trailing = `--tac-titlebar-trailing-inset: ${insets.trailing}px;`
    this._sheet.replaceSync(`:root { ${height} ${leading} ${trailing} }`)
  }

  async request() {
    try {
      this.apply(JSON.parse(await globalThis.__tachyonNativeHostCall('window.titleBarInsets', '{}')))
    } catch {
      this.apply({ height: 0, leading: 0, trailing: 0 })
    }
  }
})()
