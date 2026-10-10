class ThemeDefault {
  /** @type {string} */
  static _KEY = 'w-theme'
  /** @type {string} */
  static _ATTRIBUTE = 'w-theme'
  /** @type {string} */
  static _THEME = 'dark'

  /** @returns {void} */
  static save() {
    if (localStorage.getItem(ThemeDefault._KEY) === null) localStorage.setItem(ThemeDefault._KEY, ThemeDefault._THEME)
  }

  /** @returns {void} */
  static show() {
    const theme = localStorage.getItem(ThemeDefault._KEY) ?? ThemeDefault._THEME
    document.documentElement.setAttribute(ThemeDefault._ATTRIBUTE, theme)
  }
}

ThemeDefault.save()
ThemeDefault.show()
