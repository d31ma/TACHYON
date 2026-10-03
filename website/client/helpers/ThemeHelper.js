@Helper
export default class ThemeHelper {
  /** @type {string} */
  static _KEY = 'w-theme'
  /** @type {string} */
  static _DEFAULT = 'dark'

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
      return localStorage.getItem(ThemeHelper._KEY)
    } catch {
      return null
    }
  }

  /** @param {string | null} stored @returns {string} */
  #defaultTheme(stored) {
    return stored ?? ThemeHelper._DEFAULT
  }

  /** @param {string} theme @returns {void} */
  #remember(theme) {
    try {
      localStorage.setItem(ThemeHelper._KEY, theme)
    } catch {
    }
  }

  /** @param {string} theme @returns {void} */
  #apply(theme) {
    document.documentElement.setAttribute(ThemeHelper._KEY, theme)
  }
}
