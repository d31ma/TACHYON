#!/usr/bin/env node
// @ts-check
import { spawnSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

/** @typedef {{commit: string, bundler: string, flags: string[], modules: string[], inputs: Record<string, string>, output: string}} SiteBuild */
/** @typedef {{build: SiteBuild, assets: Record<string, {sha256: string}>}} Provenance */

class DuvayBuild {
  /** @type {string} */
  static _website = fileURLToPath(new URL('..', import.meta.url))
  /** @type {string} */
  static _provenance = path.join(DuvayBuild._website, 'client/shared/assets/duvay/PROVENANCE.json')

  /** @returns {void} */
  static run() {
    const source = process.env.DUVAY_SOURCE
    if (!source) throw new Error('Set DUVAY_SOURCE to a DuVay checkout at the commit named in PROVENANCE.json.')
    const provenancePath = path.resolve(process.argv[2] ?? DuvayBuild._provenance)
    const client = path.resolve(path.dirname(provenancePath), '../../..')
    const provenance = /** @type {Provenance} */ (JSON.parse(readFileSync(provenancePath, 'utf8')))
    const build = provenance.build
    DuvayBuild._verifyInputs(source, build.inputs)
    const output = path.join(client, build.output)
    DuvayBuild._bundle(source, build, output)
    const actual = DuvayBuild._sha(readFileSync(output))
    const expected = provenance.assets['/' + build.output].sha256
    if (actual !== expected) throw new Error(`${build.output} is ${actual}; PROVENANCE.json and manifest.json expect ${expected}. Check the bundler version (${build.bundler}).`)
    console.log(`${build.output} rebuilt from DuVay ${build.commit}: ${actual}`)
  }

  /** @param {string} source @param {Record<string, string>} inputs @returns {void} */
  static _verifyInputs(source, inputs) {
    for (const [file, expected] of Object.entries(inputs)) {
      const actual = DuvayBuild._sha(readFileSync(path.join(source, file)))
      if (actual !== expected) throw new Error(`${file} in DUVAY_SOURCE is ${actual}, not the pinned ${expected}.`)
    }
  }

  /** @param {string} source @param {SiteBuild} build @param {string} output @returns {void} */
  static _bundle(source, build, output) {
    const folder = mkdtempSync(path.join(tmpdir(), 'duvay-build-'))
    try {
      for (const file of Object.keys(build.inputs)) cpSync(path.join(source, file), path.join(folder, file))
      const entry = path.join(folder, 'entry.mjs')
      writeFileSync(entry, build.modules.map(module => `import ${JSON.stringify('./' + module)}\n`).join(''))
      const result = spawnSync('bun', ['build', entry, ...build.flags, '--outfile', output], { encoding: 'utf8' })
      if (result.status !== 0) throw new Error(result.stderr || result.stdout || 'bun build failed')
    } finally {
      rmSync(folder, { recursive: true, force: true })
    }
  }

  /** @param {Uint8Array} bytes @returns {string} */
  static _sha(bytes) {
    return createHash('sha256').update(bytes).digest('hex')
  }
}

DuvayBuild.run()
