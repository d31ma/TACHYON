/** @import { SnippetFile } from '/models/SnippetFile.ts' */
export class PlatformUtils {
  /** @returns {{ id: string, label: string }[]} */
  static list() {
    return PlatformUtils.#choices()
  }

  /** @param {string} name @returns {string[] | null} */
  static reachOf(name) {
    const reach = PlatformUtils.#knownReach(PlatformUtils.#extension(name))
    return PlatformUtils.#forRealm(PlatformUtils.#isServer(name), reach)
  }

  /** @param {string} name @returns {boolean} */
  static isCompanion(name) {
    return PlatformUtils.#companionExtensions().includes(PlatformUtils.#extension(name))
  }

  /** @param {string} name @returns {number} */
  static rank(name) {
    const order = PlatformUtils.#readingOrder()
    return PlatformUtils.#knownPosition(order.indexOf(PlatformUtils.#extension(name)), order.length)
  }

  /** @param {SnippetFile[]} files @returns {SnippetFile[]} */
  static inReadingOrder(files) {
    return PlatformUtils.#sorted(files)
  }

  /** @param {string} name @returns {string | null} */
  static languageOf(name) {
    return PlatformUtils.#knownLanguage(PlatformUtils.#extension(name))
  }

  /** @param {string} name @returns {string} */
  static languageLabel(name) {
    return PlatformUtils.#label(PlatformUtils.languageOf(name))
  }

  /** @param {SnippetFile[]} files @returns {boolean} */
  static spansPlatforms(files) {
    return PlatformUtils.#distinctReaches(files).size > 1
  }

  /** @param {SnippetFile[]} files @param {string} platform @returns {SnippetFile[]} */
  static filesFor(files, platform) {
    return PlatformUtils.#availableFiles(files, platform)
  }

  /** @param {string} agent @returns {string} */
  static detectPlatform(agent) {
    return PlatformUtils.#matchedPlatform(agent)
  }

  /** @param {string | null} chosen @param {string} fallback @returns {string} */
  static knownPlatform(chosen, fallback) {
    return PlatformUtils.#choiceOrDefault(PlatformUtils.#isKnown(chosen), chosen, fallback)
  }

  /** @returns {{ id: string, label: string }[]} */
  static #choices() {
    return [
      { id: 'web', label: 'Web' },
      { id: 'macos', label: 'macOS' },
      { id: 'ios', label: 'iOS' },
      { id: 'android', label: 'Android' },
      { id: 'windows', label: 'Windows' },
      { id: 'linux', label: 'Linux' },
    ]
  }

  /** @returns {string[]} */
  static #allPlatforms() {
    return ['web', 'macos', 'ios', 'android', 'windows', 'linux']
  }

  /** @returns {Record<string, string[]>} */
  static #reaches() {
    return {
    js: PlatformUtils.#allPlatforms(),
    ts: PlatformUtils.#allPlatforms(),
    dart: PlatformUtils.#allPlatforms(),
    py: ['web'],
    rs: PlatformUtils.#allPlatforms(),
    cs: PlatformUtils.#allPlatforms(),
    fs: PlatformUtils.#allPlatforms(),
    go: PlatformUtils.#allPlatforms(),
    rb: PlatformUtils.#allPlatforms(),
    lua: PlatformUtils.#allPlatforms(),
    java: PlatformUtils.#allPlatforms(),
    html: PlatformUtils.#allPlatforms(),
    css: PlatformUtils.#allPlatforms(),
    swift: ['macos', 'ios'],
    kt: ['android'],
  }

  }

  /** @returns {Record<string, string>} */
  static #languageNames() {
    return {
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
    lua: 'Lua',
    go: 'Go',
  }

  }

  /** @returns {string[]} */
  static #companionExtensions() {
    return ['js', 'ts', 'dart', 'swift', 'kt', 'cs', 'fs', 'rs', 'go', 'rb', 'lua', 'py', 'java', 'groovy', 'php']
  }

  /** @returns {string[]} */
  static #readingOrder() {
    return ['js', 'ts', 'dart', 'py', 'java', 'groovy', 'php', 'swift', 'kt', 'cs', 'fs', 'rs', 'go', 'rb', 'lua', 'html', 'css']
  }

  /** @param {string} name @returns {string} */
  static #extension(name) {
    return String(name).split('.').pop()?.toLowerCase() ?? ''
  }

  /** @param {string} name @returns {boolean} */
  static #isServer(name) {
    return String(name).startsWith('server/') || String(name).startsWith('middleware.')
  }

  /** @param {string} extension @returns {string[] | null} */
  static #knownReach(extension) {
    return PlatformUtils.#reaches()[extension] ?? null
  }

  /** @param {boolean} server @param {string[] | null} reach @returns {string[] | null} */
  static #forRealm(server, reach) {
    return server ? null : reach
  }

  /** @param {number} position @param {number} length @returns {number} */
  static #knownPosition(position, length) {
    return position === -1 ? length : position
  }

  /** @param {SnippetFile[]} files @returns {SnippetFile[]} */
  static #sorted(files) {
    return [...files].sort(PlatformUtils.#compareFiles)
  }

  /** @param {SnippetFile} left @param {SnippetFile} right @returns {number} */
  static #compareFiles(left, right) {
    return PlatformUtils.rank(left.name) - PlatformUtils.rank(right.name)
  }

  /** @param {string} extension @returns {string | null} */
  static #knownLanguage(extension) {
    return PlatformUtils.#languageNames()[extension] ?? null
  }

  /** @param {string | null} language @returns {string} */
  static #label(language) {
    return language ?? ''
  }

  /** @param {SnippetFile[]} files @returns {Set<string>} */
  static #distinctReaches(files) {
    return new Set(files.map(PlatformUtils.#reachKey).filter(Boolean))
  }

  /** @param {SnippetFile} file @returns {string} */
  static #reachKey(file) {
    return PlatformUtils.reachOf(file.name)?.join(',') ?? ''
  }

  /** @param {SnippetFile[]} files @param {string} platform @returns {SnippetFile[]} */
  static #availableFiles(files, platform) {
    return files.filter((file) => PlatformUtils.#reachesPlatform(PlatformUtils.reachOf(file.name), platform))
  }

  /** @param {string[] | null} reach @param {string} platform @returns {boolean} */
  static #reachesPlatform(reach, platform) {
    return reach === null || reach.includes(platform)
  }

  /** @param {string} agent @returns {string} */
  static #matchedPlatform(agent) {
    return PlatformUtils.#platformPatterns().find(([pattern]) => pattern.test(agent))?.[1] ?? 'web'
  }

  /** @returns {[RegExp, string][]} */
  static #platformPatterns() {
    return [[/iPhone|iPad|iPod/, 'ios'], [/Android/, 'android'], [/Macintosh/, 'macos'], [/Windows/, 'windows'], [/Linux/, 'linux']]
  }

  /** @param {string | null} chosen @returns {boolean} */
  static #isKnown(chosen) {
    return PlatformUtils.list().some((platform) => platform.id === chosen)
  }

  /** @param {boolean} known @param {string | null} chosen @param {string} fallback @returns {string} */
  static #choiceOrDefault(known, chosen, fallback) {
    return known ? String(chosen) : fallback
  }
}
