class DocsDataPreload {
  /** @type {string} */
  static _DATA = '/shared/data/docs.json'
  /** @type {string} */
  static _DOCS = 'docs'
  /** @type {string} */
  static _FEATURES = 'features'

  /** @returns {void} */
  static start() {
    if (DocsDataPreload.onGuidePage(location.pathname)) DocsDataPreload.load()
  }

  /**
   * @param {string} path
   * @returns {boolean}
   */
  static onGuidePage(path) {
    const parts = path.split('/').filter(Boolean)
    return parts.length === 2 && parts[0] === DocsDataPreload._DOCS && parts[1] !== DocsDataPreload._FEATURES
  }

  /** @returns {void} */
  static load() {
    import(DocsDataPreload._DATA, { with: { type: 'json' } }).catch(() => undefined)
  }
}

DocsDataPreload.start()
