/** @import { SnippetFile } from '/models/SnippetFile.ts' */
export class PlatformUtils {
  /** @returns {{ id: string, label: string }[]} */
  static list() {
    return PlatformUtils._choices().map(PlatformUtils._choiceOf)
  }

  /** @param {string} name @returns {string[] | null} */
  static _reachOf(name) {
    const reach = PlatformUtils._knownReach(PlatformUtils._extension(name))
    return PlatformUtils._forRealm(PlatformUtils._isServer(name), reach)
  }

  /** @param {string} name @returns {boolean} */
  static isCompanion(name) {
    return PlatformUtils._companionExtensions().includes(PlatformUtils._extension(name))
  }

  /** @param {string} name @returns {number} */
  static _rank(name) {
    const order = PlatformUtils._readingOrder()
    return PlatformUtils._knownPosition(order.indexOf(PlatformUtils._extension(name)), order.length)
  }

  /** @param {SnippetFile[]} files @returns {SnippetFile[]} */
  static inReadingOrder(files) {
    return PlatformUtils._sorted(files)
  }

  /** @param {string} name @returns {string | null} */
  static _languageOf(name) {
    return PlatformUtils._knownLanguage(PlatformUtils._extension(name))
  }

  /** @param {string} name @returns {string} */
  static languageLabel(name) {
    return PlatformUtils._label(PlatformUtils._languageOf(name))
  }

  /** @param {SnippetFile[]} files @returns {boolean} */
  static spansPlatforms(files) {
    return PlatformUtils._distinctReaches(files).size > 1
  }

  /** @param {SnippetFile[]} files @param {string} platform @returns {SnippetFile[]} */
  static filesFor(files, platform) {
    return PlatformUtils._availableFiles(files, platform)
  }

  /** @param {string} agent @returns {string} */
  static detectPlatform(agent) {
    return PlatformUtils._matchedPlatform(agent)
  }

  /** @param {string | null} chosen @param {string} fallback @returns {string} */
  static knownPlatform(chosen, fallback) {
    return PlatformUtils._choiceOrDefault(PlatformUtils._isKnown(chosen), chosen, fallback)
  }

  /** @returns {string} */
  static _web() {
    return 'web'
  }

  /** @returns {{ id: string, label: string, agent: RegExp | null }[]} */
  static _choices() {
    return [
      { id: PlatformUtils._web(), label: 'Web', agent: null },
      { id: 'macos', label: 'macOS', agent: /Macintosh/ },
      { id: 'ios', label: 'iOS', agent: /iPhone|iPad|iPod/ },
      { id: 'android', label: 'Android', agent: /Android/ },
      { id: 'windows', label: 'Windows', agent: /Windows/ },
      { id: 'linux', label: 'Linux', agent: /Linux/ },
    ]
  }

  /** @param {{ id: string, label: string }} choice @returns {{ id: string, label: string }} */
  static _choiceOf(choice) {
    return { id: choice.id, label: choice.label }
  }

  /** @returns {string[]} */
  static _allPlatforms() {
    return PlatformUtils._choices().map((choice) => choice.id)
  }

  /** @returns {Record<string, string[]>} */
  static _reaches() {
    return {
      js: PlatformUtils._allPlatforms(),
      ts: PlatformUtils._allPlatforms(),
      dart: PlatformUtils._allPlatforms(),
      py: [PlatformUtils._web()],
      rs: PlatformUtils._allPlatforms(),
      cs: PlatformUtils._allPlatforms(),
      fs: PlatformUtils._allPlatforms(),
      go: PlatformUtils._allPlatforms(),
      rb: PlatformUtils._allPlatforms(),
      lua: PlatformUtils._allPlatforms(),
      java: PlatformUtils._allPlatforms(),
      html: PlatformUtils._allPlatforms(),
      css: PlatformUtils._allPlatforms(),
      swift: PlatformUtils._allPlatforms(),
      kt: PlatformUtils._allPlatforms(),
    }
  }

  /** @returns {Record<string, string>} */
  static _languageNames() {
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
      hpp: 'C++',
      go: 'Go',
    }
  }

  /** @returns {string[]} */
  static _companionExtensions() {
    return ['js', 'ts', 'dart', 'py', 'java', 'groovy', 'php', 'swift', 'kt', 'cs', 'fs', 'rs', 'go', 'rb', 'lua', 'hpp']
  }

  /** @returns {string[]} */
  static _readingOrder() {
    return [...PlatformUtils._companionExtensions(), 'html', 'css']
  }

  /** @param {string} name @returns {string} */
  static _extension(name) {
    return String(name).split('.').pop()?.toLowerCase() ?? ''
  }

  /** @param {string} name @returns {boolean} */
  static _isServer(name) {
    return String(name).startsWith('server/') || String(name).startsWith('middleware.')
  }

  /** @param {string} extension @returns {string[] | null} */
  static _knownReach(extension) {
    return PlatformUtils._reaches()[extension] ?? null
  }

  /** @param {boolean} server @param {string[] | null} reach @returns {string[] | null} */
  static _forRealm(server, reach) {
    return server ? null : reach
  }

  /** @param {number} position @param {number} length @returns {number} */
  static _knownPosition(position, length) {
    return position === -1 ? length : position
  }

  /** @param {SnippetFile[]} files @returns {SnippetFile[]} */
  static _sorted(files) {
    return [...files].sort(PlatformUtils._compareFiles)
  }

  /** @param {SnippetFile} left @param {SnippetFile} right @returns {number} */
  static _compareFiles(left, right) {
    return PlatformUtils._rank(left.name) - PlatformUtils._rank(right.name)
  }

  /** @param {string} extension @returns {string | null} */
  static _knownLanguage(extension) {
    return PlatformUtils._languageNames()[extension] ?? null
  }

  /** @param {string | null} language @returns {string} */
  static _label(language) {
    return language ?? ''
  }

  /** @param {SnippetFile[]} files @returns {Set<string>} */
  static _distinctReaches(files) {
    return new Set(files.map(PlatformUtils._reachKey).filter(Boolean))
  }

  /** @param {SnippetFile} file @returns {string} */
  static _reachKey(file) {
    return PlatformUtils._reachOf(file.name)?.join(',') ?? ''
  }

  /** @param {SnippetFile[]} files @param {string} platform @returns {SnippetFile[]} */
  static _availableFiles(files, platform) {
    return files.filter((file) => PlatformUtils._reachesPlatform(PlatformUtils._reachOf(file.name), platform))
  }

  /** @param {string[] | null} reach @param {string} platform @returns {boolean} */
  static _reachesPlatform(reach, platform) {
    return reach === null || reach.includes(platform)
  }

  /** @param {string} agent @returns {string} */
  static _matchedPlatform(agent) {
    return PlatformUtils._choices().find((choice) => choice.agent?.test(agent))?.id ?? PlatformUtils._web()
  }

  /** @param {string | null} chosen @returns {boolean} */
  static _isKnown(chosen) {
    return PlatformUtils.list().some((platform) => platform.id === chosen)
  }

  /** @param {boolean} known @param {string | null} chosen @param {string} fallback @returns {string} */
  static _choiceOrDefault(known, chosen, fallback) {
    return known ? String(chosen) : fallback
  }
}
