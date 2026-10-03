@Helper
export default class ThemeHelper {
  /** @type {string} */
  static KEY = 'w-theme'
  /** @type {string} */
  static DEFAULT = 'dark'

  /** @returns {string} */
  restore() {
    const theme = this.#defaultTheme(this.#storedTheme())
    this.#remember(theme)
    this.#apply(theme)
    return theme
  }

  /** @returns {string | null} */
  #storedTheme() {
    try {
      return localStorage.getItem(ThemeHelper.KEY)
    } catch {
      return null
    }
  }

  /** @param {string | null} stored @returns {string} */
  #defaultTheme(stored) {
    return stored ?? ThemeHelper.DEFAULT
  }

  /** @param {string} theme @returns {void} */
  #remember(theme) {
    try {
      localStorage.setItem(ThemeHelper.KEY, theme)
    } catch {
    }
  }

  /** @param {string} theme @returns {void} */
  #apply(theme) {
    document.documentElement.setAttribute(ThemeHelper.KEY, theme)
  }
}
