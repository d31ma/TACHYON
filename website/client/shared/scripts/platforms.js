// @ts-nocheck

export class Platforms {
  static list = [
    { id: 'web', label: 'Web' },
    { id: 'macos', label: 'macOS' },
    { id: 'ios', label: 'iOS' },
    { id: 'android', label: 'Android' },
    { id: 'windows', label: 'Windows' },
    { id: 'linux', label: 'Linux' },
  ]

  static storageKey = 'tachyon.platform'
}

class FileTypes {
  static allPlatforms = ['web', 'macos', 'ios', 'android', 'windows', 'linux']

  static platformsByExtension = {
    js: FileTypes.allPlatforms,
    ts: FileTypes.allPlatforms,
    dart: FileTypes.allPlatforms,
    rs: FileTypes.allPlatforms,
    cs: FileTypes.allPlatforms,
    fs: FileTypes.allPlatforms,
    go: FileTypes.allPlatforms,
    rb: FileTypes.allPlatforms,
    html: FileTypes.allPlatforms,
    css: FileTypes.allPlatforms,
    swift: ['macos', 'ios'],
    kt: ['android'],
  }

  static languageNames = {
    html: 'View',
    css: 'Styles',
    js: 'JavaScript',
    ts: 'TypeScript',
    dart: 'Dart',
    py: 'Python',
    java: 'Java',
    groovy: 'Groovy',
    php: 'PHP',
    swift: 'Swift',
    kt: 'Kotlin',
    cs: 'C#',
    fs: 'F#',
    rs: 'Rust',
    rb: 'Ruby',
    go: 'Go',
  }

  static companionExtensions = new Set(['js', 'ts', 'dart', 'swift', 'kt', 'cs', 'fs', 'rs', 'go', 'rb', 'py', 'java', 'groovy', 'php'])

  static readingOrder = ['js', 'ts', 'dart', 'py', 'java', 'groovy', 'php', 'swift', 'kt', 'cs', 'fs', 'rs', 'go', 'rb', 'html', 'css']
}

function extension(name) {
  return String(name).split('.').pop()?.toLowerCase() ?? ''
}

function isServer(name) {
  return String(name).startsWith('server/') || String(name).startsWith('middleware.')
}

export function reachOf(name) {
  if (isServer(name)) return null
  return FileTypes.platformsByExtension[extension(name)] ?? null
}

export function isCompanion(name) {
  return FileTypes.companionExtensions.has(extension(name))
}

export function rank(name) {
  const position = FileTypes.readingOrder.indexOf(extension(name))
  if (position === -1) return FileTypes.readingOrder.length
  return position
}

export function inReadingOrder(files) {
  return [...files].sort((left, right) => rank(left.name) - rank(right.name))
}

export function languageOf(name) {
  return FileTypes.languageNames[extension(name)] ?? null
}

export function languageLabel(name) {
  return languageOf(name) ?? ''
}

export function spansPlatforms(files) {
  const distinctReaches = new Set()
  for (const file of files) {
    const reach = reachOf(file.name)
    if (reach) distinctReaches.add(reach.join(','))
  }
  return distinctReaches.size > 1
}

export function filesFor(files, platform) {
  return files.filter((file) => {
    const reach = reachOf(file.name)
    return reach === null || reach.includes(platform)
  })
}

export function storedPlatform() {
  try {
    const chosen = localStorage.getItem(Platforms.storageKey)
    if (chosen && Platforms.list.some((platform) => platform.id === chosen)) return chosen
  } catch {
  }
  return detectPlatform()
}

export function rememberPlatform(platform) {
  try {
    localStorage.setItem(Platforms.storageKey, platform)
  } catch {
  }
}

export function detectPlatform() {
  const agent = navigator.userAgent
  if (/iPhone|iPad|iPod/.test(agent)) return 'ios'
  if (/Android/.test(agent)) return 'android'
  if (/Macintosh/.test(agent)) return 'macos'
  if (/Windows/.test(agent)) return 'windows'
  if (/Linux/.test(agent)) return 'linux'
  return 'web'
}
