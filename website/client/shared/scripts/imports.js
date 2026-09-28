function isInBrowser() {
  return typeof document !== 'undefined' && typeof document.querySelector === 'function'
}

function addDuvayScripts() {
  if (document.querySelector('script[data-duvay]')) return
  for (const [file, marker] of [['duvay-wc.min.js', 'wc'], ['duvay.min.js', 'behaviours']]) {
    const script = document.createElement('script')
    script.src = new URL(`../assets/duvay/${file}`, import.meta.url).href
    script.defer = true
    script.dataset.duvay = marker
    document.head.append(script)
  }
}

function applyStoredTheme() {
  try {
    const stored = localStorage.getItem('w-theme') ?? 'dark'
    localStorage.setItem('w-theme', stored)
    document.documentElement.setAttribute('w-theme', stored)
  } catch {
    document.documentElement.setAttribute('w-theme', 'dark')
  }
}

if (isInBrowser()) {
  addDuvayScripts()
  applyStoredTheme()
}
