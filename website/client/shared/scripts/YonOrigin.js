class YonOrigin {
  /** @type {Record<string, string>} */
  static _ORIGINS = {
    'tachyon.del.ma': 'https://api.tachyon.del.ma',
    'staging.tachyon.del.ma': 'https://api.staging.tachyon.del.ma',
  }
  /** @type {string} */
  static _TAG = 'meta[name="tachyon-yon-origin"]'
  /** @type {string} */
  static _CONTENT = 'content'

  /**
   * @param {string} hostname
   * @returns {string | undefined}
   */
  static forHost(hostname) {
    return Object.hasOwn(YonOrigin._ORIGINS, hostname) ? YonOrigin._ORIGINS[hostname] : undefined
  }

  /** @returns {void} */
  static choose() {
    const tag = document.querySelector(YonOrigin._TAG)
    if (tag === null) return
    YonOrigin.point(tag, YonOrigin.forHost(location.hostname))
  }

  /**
   * @param {Element} tag
   * @param {string | undefined} origin
   * @returns {void}
   */
  static point(tag, origin) {
    if (origin === undefined) {
      tag.remove()
      return
    }
    tag.setAttribute(YonOrigin._CONTENT, origin)
  }
}

YonOrigin.choose()
