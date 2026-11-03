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
    if (extension !== PythonWebRuntime.extension) return files
    const hasManifest = files.some((file) => file.name === PythonWebRuntime.manifestName)
    if (!hasManifest) return [...files, { name: PythonWebRuntime.manifestName, code: PythonWebRuntime.manifest }]
    return files.map((file) => file.name === PythonWebRuntime.manifestName ? { name: file.name, code: PythonWebRuntime.merged(file.code) } : file)
  }

  /** @param {string} code @returns {string} */
  static merged(code) {
    const written = JSON.parse(code)
    const python = JSON.parse(PythonWebRuntime.manifest).tac.python
    return JSON.stringify({ ...written, tac: { python, ...written.tac } })
  }

  /** @returns {string} */
  static cacheRoot() {
    if (process.env.TAC_CACHE_DIR) return process.env.TAC_CACHE_DIR
    if (process.platform === 'darwin') return path.join(homedir(), 'Library/Caches/Tachyon')
    if (process.platform === 'win32') return path.join(process.env.LOCALAPPDATA ?? path.join(homedir(), 'AppData/Local'), 'Tachyon/Cache')
    return path.join(process.env.XDG_CACHE_HOME || path.join(homedir(), '.cache'), 'tachyon')
  }
}
