#!/usr/bin/env -S node --experimental-wasm-jspi
import { readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { pathToFileURL } from 'node:url'

class PyodideTestUnit {
  static async main() {
    const core = path.join(process.env.TAC_PYTHON_CORE, path.sep)
    const loader = await import(pathToFileURL(path.join(core, 'pyodide.mjs')).href)
    const runtime = await loader.loadPyodide({
      indexURL: core,
      lockFileURL: path.join(core, 'pyodide-lock.json'),
      stdLibURL: path.join(core, 'python_stdlib.zip'),
      stdout: (line) => process.stdout.write(`${line}\n`),
      stderr: (line) => process.stderr.write(`${line}\n`),
    })
    await runtime.runPythonAsync(readFileSync(process.argv[2], 'utf8'))
    PyodideTestUnit.receipt(runtime)
  }

  static receipt(runtime) {
    const platform = runtime.runPython('import sys\nsys.platform')
    const receipt = { platform, pyodide: runtime.version }
    writeFileSync(process.env.TAC_PYTHON_UNIT_RECEIPT, JSON.stringify(receipt))
  }
}

await PyodideTestUnit.main()
