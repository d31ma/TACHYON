import { existsSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'

/** @typedef {{name:string,code:string}} Snippet */

export class PythonWebRuntime {
  static extension = '.py'
  static version = '314.0.7'
  static wasm = 'pyodide.asm.wasm'
  static manifestName = 'client/manifest.json'
  static manifest = JSON.stringify({ tac: { python: { runtime: `pyodide-${PythonWebRuntime.version}`, profile: 'offline' } } })

  /** @param {string} extension @returns {boolean} */
  static isMissing(extension) {
    if (extension !== PythonWebRuntime.extension) return false
    return !existsSync(path.join(PythonWebRuntime.cacheRoot(), 'python-web', PythonWebRuntime.version, PythonWebRuntime.wasm))
  }

  /** @param {Snippet[]} files @param {string} extension @returns {Snippet[]} */
  static withOptIn(files, extension) {
    const hasManifest = files.some((file) => file.name === PythonWebRuntime.manifestName)
    if (extension !== PythonWebRuntime.extension || hasManifest) return files
    return [...files, { name: PythonWebRuntime.manifestName, code: PythonWebRuntime.manifest }]
  }

  /** @returns {string} */
  static cacheRoot() {
    if (process.env.TAC_CACHE_DIR) return process.env.TAC_CACHE_DIR
    if (process.platform === 'darwin') return path.join(homedir(), 'Library/Caches/Tachyon')
    if (process.platform === 'win32') return path.join(process.env.LOCALAPPDATA ?? path.join(homedir(), 'AppData/Local'), 'Tachyon/Cache')
    return path.join(process.env.XDG_CACHE_HOME || path.join(homedir(), '.cache'), 'tachyon')
  }
}
