@Test(SiteTestSetup, {"partition":"tests"})
export default class ThemeHelperTest {
  /** @returns {Promise<void>} */
  async testRestoreRestoresSavedTheme() {
    const saved = localStorage.getItem('w-theme')
    const root = document.documentElement.getAttribute('w-theme')
    try {
      localStorage.setItem('w-theme', 'light')
      const theme = new ThemeHelper()
      const current = theme.restore()
      Assert.equals('light', current)
      Assert.equals('light', document.documentElement.getAttribute('w-theme'))
    } finally {
      this.restore(saved, root)
    }
  }

  /** @returns {Promise<void>} */
  async testRestoreSetsDarkOnFirstVisit() {
    const saved = localStorage.getItem('w-theme')
    const root = document.documentElement.getAttribute('w-theme')
    try {
      localStorage.removeItem('w-theme')
      const theme = new ThemeHelper()
      const current = theme.restore()
      Assert.equals('dark', current)
      Assert.equals('dark', localStorage.getItem('w-theme'))
      Assert.equals('dark', document.documentElement.getAttribute('w-theme'))
    } finally {
      this.restore(saved, root)
    }
  }

  /** @param {string | null} saved @param {string | null} root @returns {void} */
  restore(saved, root) {
    if (saved === null) localStorage.removeItem('w-theme')
    else localStorage.setItem('w-theme', saved)
    if (root === null) document.documentElement.removeAttribute('w-theme')
    else document.documentElement.setAttribute('w-theme', root)
  }
}
