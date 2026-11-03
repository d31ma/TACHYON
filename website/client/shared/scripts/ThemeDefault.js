class ThemeDefault {
  /** @type {string} */
  static _KEY = 'w-theme'
  /** @type {string} */
  static _ATTRIBUTE = 'w-theme'
  /** @type {string} */
  static _THEME = 'dark'

  /** @returns {string | null} */
  static saved() {
    try {
      return localStorage.getItem(ThemeDefault._KEY)
    } catch {
      return null
    }
  }

  /** @returns {void} */
  static save() {
    if (ThemeDefault.saved() !== null) return
    try {
      localStorage.setItem(ThemeDefault._KEY, ThemeDefault._THEME)
    } catch {
      return
    }
  }

  /** @returns {void} */
  static show() {
    const theme = ThemeDefault.saved() ?? ThemeDefault._THEME
    document.documentElement.setAttribute(ThemeDefault._ATTRIBUTE, theme)
  }
}

ThemeDefault.save()
ThemeDefault.show()
