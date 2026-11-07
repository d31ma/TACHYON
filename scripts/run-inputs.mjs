import { execFileSync } from 'node:child_process'
import { mkdtemp } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from './cargo-artifacts.mjs'

export class RunInputs {
  static repository = fileURLToPath(new URL('../', import.meta.url))

  /** @returns {string} */
  static binary() {
    return path.resolve(process.env.TAC_BIN ?? CargoArtifacts.path('release', process.platform === 'win32' ? 'ty.exe' : 'ty'))
  }

  /**
   * @param {string} name
   * @returns {string}
   */
  static sourceCommit(name) {
    return process.env[name] ?? execFileSync('git', ['rev-parse', 'HEAD'], { cwd: RunInputs.repository, encoding: 'utf8' }).trim()
  }

  /**
   * @param {string} name
   * @param {string} prefix
   * @returns {Promise<string>}
   */
  static async folder(name, prefix) {
    return path.resolve(process.env[name] ?? await mkdtemp(path.join(tmpdir(), prefix)))
  }
}
