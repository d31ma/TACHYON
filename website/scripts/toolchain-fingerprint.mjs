import { execFile } from 'node:child_process'
import { createHash } from 'node:crypto'
import { createReadStream, existsSync, readdirSync, readFileSync } from 'node:fs'
import path from 'node:path'

/** @typedef {[string, string[]]} VersionCommand */

export class ToolchainFingerprint {
  static versionTimeoutMilliseconds = 30_000
  static versionOutputBytes = 1024 * 1024
  static unavailable = 'unavailable'
  static ignoredSettings = new Set(['TAC_BIN', 'TAC_CACHE_DIR', 'TAC_SNIPPETS_CONCURRENCY', 'TAC_SNIPPETS_SCRATCH'])
  static settingPrefixes = ['TAC_', 'YON_']
  static otherSettings = ['RUSTUP_TOOLCHAIN', 'JAVA_HOME', 'DOTNET_ROOT', 'GOROOT', 'EM_CONFIG']
  static inputFiles = [
    'Cargo.lock',
    'rust-toolchain.toml',
    'package.json',
    'bun.lock',
    'website/package.json',
    'website/bun.lock',
    'scripts/release/server-probe.mjs',
  ]
  static verifierFolder = 'website/scripts'

  /** @returns {VersionCommand[]} */
  static versionCommands() {
    const swiftCompiler = process.env.TAC_SWIFT_WASM_COMPILER ?? 'swiftc'
    return [
      ['rustc', ['-vV']],
      ['rustup', ['target', 'list', '--installed']],
      ['dotnet', ['--version']],
      ['dotnet', ['workload', 'list']],
      [swiftCompiler, ['--version']],
      ['go', ['version']],
      ['tinygo', ['version']],
      ['emcc', ['--version']],
      ['dart', ['--version']],
      ['kotlinc', ['-version']],
      ['kotlinc-js', ['-version']],
      ['javac', ['-version']],
      ['java', ['-version']],
      ['groovyc', ['--version']],
      ['bun', ['--version']],
      ['node', ['--version']],
      ['python3', ['--version']],
      ['php', ['--version']],
      ['ruby', ['--version']],
      ['lua', ['-v']],
      ['luac', ['-v']],
      ['em++', ['--version']],
      ['clang++', ['--version']],
      ['tsc', ['--version']],
    ]
  }

  /**
   * @param {string} repo
   * @param {string} binary
   * @returns {Promise<string>}
   */
  static async read(repo, binary) {
    const versions = await Promise.all(ToolchainFingerprint.versionCommands().map(ToolchainFingerprint.version))
    const parts = [
      `ty ${await ToolchainFingerprint.fileDigest(binary)}`,
      ...ToolchainFingerprint.inputDigests(repo),
      ...ToolchainFingerprint.settings(process.env),
      ...versions,
    ]
    return parts.join('\n')
  }

  /**
   * @param {VersionCommand} command
   * @returns {Promise<string>}
   */
  static version([tool, args]) {
    const options = { timeout: ToolchainFingerprint.versionTimeoutMilliseconds, maxBuffer: ToolchainFingerprint.versionOutputBytes }
    return new Promise(resolve => {
      execFile(tool, args, options, (error, stdout, stderr) => {
        const answer = error ? ToolchainFingerprint.unavailable : `${stdout}${stderr}`.trim()
        resolve(`${tool} ${args.join(' ')}: ${answer}`)
      })
    })
  }

  /**
   * @param {string} file
   * @returns {Promise<string>}
   */
  static fileDigest(file) {
    return new Promise((resolve, reject) => {
      const hash = createHash('sha256')
      createReadStream(file)
        .on('data', chunk => hash.update(chunk))
        .on('error', reject)
        .on('end', () => resolve(hash.digest('hex')))
    })
  }

  /**
   * @param {string} repo
   * @returns {string[]}
   */
  static inputDigests(repo) {
    const verifierFolder = path.join(repo, ToolchainFingerprint.verifierFolder)
    const verifierFiles = readdirSync(verifierFolder).sort()
      .filter(name => name.endsWith('.mjs'))
      .map(name => path.join(ToolchainFingerprint.verifierFolder, name))
    return [...ToolchainFingerprint.inputFiles, ...verifierFiles].map(name => {
      const file = path.join(repo, name)
      const digest = existsSync(file) ? createHash('sha256').update(readFileSync(file)).digest('hex') : 'missing'
      return `${name} ${digest}`
    })
  }

  /**
   * @param {NodeJS.ProcessEnv} environment
   * @returns {string[]}
   */
  static settings(environment) {
    return Object.keys(environment)
      .filter(name => ToolchainFingerprint.affectsResults(name))
      .sort()
      .map(name => `${name}=${environment[name]}`)
  }

  /**
   * @param {string} name
   * @returns {boolean}
   */
  static affectsResults(name) {
    if (ToolchainFingerprint.ignoredSettings.has(name)) return false
    if (ToolchainFingerprint.otherSettings.includes(name)) return true
    return ToolchainFingerprint.settingPrefixes.some(prefix => name.startsWith(prefix))
  }
}
