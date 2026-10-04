class ThemeDefault {
  /** @type {string} */
  static _KEY = 'w-theme'
  /** @type {string} */
  static _THEME = 'dark'

  /** @returns {void} */
  static save() {
    if (localStorage.getItem(ThemeDefault._KEY) === null) localStorage.setItem(ThemeDefault._KEY, ThemeDefault._THEME)
  }
}

ThemeDefault.save()
