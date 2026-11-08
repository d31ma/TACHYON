#!/usr/bin/env node
import { cpSync, existsSync, mkdirSync, rmSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

class StageRunnerImage {
  static website = fileURLToPath(new URL('..', import.meta.url))
  static WEBSITE_PARTS = ['client', 'server', 'runner']
  static OPTIONS = ['--out', '--ty', '--runnable', '--website-dist']
  static BUILD_OUTPUT = ['dist', '.tachyon', 'node_modules']

  static run() {
    const options = StageRunnerImage.options(process.argv)
    rmSync(options.out, { recursive: true, force: true })
    mkdirSync(path.join(options.out, 'website', 'dist'), { recursive: true })
    for (const part of StageRunnerImage.WEBSITE_PARTS) cpSync(path.join(StageRunnerImage.website, part), path.join(options.out, 'website', part), { recursive: true })
    cpSync(options.websiteDist, path.join(options.out, 'website', 'dist', 'web'), { recursive: true })
    cpSync(options.runnable, path.join(options.out, 'runnable'), { recursive: true, filter: source => !StageRunnerImage.isBuildOutput(source) })
    cpSync(options.ty, path.join(options.out, 'ty'))
    cpSync(path.join(StageRunnerImage.website, 'runner', 'Dockerfile'), path.join(options.out, 'Dockerfile'))
    console.log(`Staged the snippet runner image context in ${options.out}`)
  }

  /**
   * @param {string} source
   * @returns {boolean}
   */
  static isBuildOutput(source) {
    return StageRunnerImage.BUILD_OUTPUT.includes(path.basename(source))
  }

  /**
   * @param {string[]} argumentsList
   * @returns {{ out: string, ty: string, runnable: string, websiteDist: string }}
   */
  static options(argumentsList) {
    const [out, ty, runnable, websiteDist] = StageRunnerImage.OPTIONS.map(name => StageRunnerImage.existingValue(argumentsList, name))
    return { out, ty, runnable, websiteDist }
  }

  /**
   * @param {string[]} argumentsList
   * @param {string} name
   * @returns {string}
   */
  static existingValue(argumentsList, name) {
    const index = argumentsList.indexOf(name)
    const value = index === -1 ? undefined : argumentsList[index + 1]
    if (value === undefined) throw new Error(`stage-runner-image needs ${StageRunnerImage.OPTIONS.map(option => `${option} <path>`).join(' ')}`)
    if (name !== '--out' && !existsSync(value)) throw new Error(`${name} ${value} does not exist`)
    return path.resolve(value)
  }
}

StageRunnerImage.run()
