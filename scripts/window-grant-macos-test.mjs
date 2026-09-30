#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

assert.equal(process.platform, 'darwin', 'The native window-grant check needs macOS.')

class WindowGrantCheck {
  static repository = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static temporary = mkdtempSync(join(tmpdir(), 'tachyon-window-grant-'))
  static helper = join(WindowGrantCheck.temporary, 'window-grant-bounds')
  static openedWidth = 420
  static widenedWidth = 480
  static attempts = 60
  static pause = 250
  static steadyWait = 3000

  static service = `@Service
export default class GrantWindowService {
  /** @type {TacWindowControls} */
  @Needs
  windowControls;
  problem = '';

  @OnStart
  widen() {
    this.#keepProblem(() => this.windowControls.resize(${WindowGrantCheck.widenedWidth}, 720));
  }

  /** @param {() => void} call */
  #keepProblem(call) {
    try {
      call();
    } catch (refusal) {
      this.problem = /** @type {Error} */ (refusal).message;
    }
  }
}
`

  static run(program, commandArguments) {
    const result = spawnSync(program, commandArguments, { encoding: 'utf8', timeout: 600_000 })
    assert.equal(result.status, 0, `${program} ${commandArguments.join(' ')} failed:\n${result.stderr || result.stdout}`)
    return result.stdout
  }

  static write(root, relative, contents) {
    const file = join(root, relative)
    mkdirSync(dirname(file), { recursive: true })
    writeFileSync(file, contents)
  }

  static project(name, controls) {
    const root = join(WindowGrantCheck.temporary, name)
    const window = { width: WindowGrantCheck.openedWidth, height: 720, controls }
    WindowGrantCheck.write(root, 'client/manifest.json', JSON.stringify({
      name, start_url: '/', tac: { application: { id: `dev.tachyon.${name.toLowerCase()}`, version: '0.1.0', window } },
    }))
    WindowGrantCheck.write(root, 'client/pages/tac.html', '<grant-window />\n')
    WindowGrantCheck.write(root, 'client/components/grant/window/tac.html', '<main aria-label="Window grant"><p>{grantWindow.problem}</p></main>\n')
    WindowGrantCheck.write(root, 'client/services/GrantWindowService.js', WindowGrantCheck.service)
    WindowGrantCheck.run(WindowGrantCheck.binary, ['build', root, '--target', 'macos'])
    const app = join(root, 'dist/macos', `${name}.app`)
    assert.ok(existsSync(app), `${app} was not built`)
    return app
  }

  static width(name) {
    const answer = spawnSync(WindowGrantCheck.helper, [name], { encoding: 'utf8' })
    return answer.status === 0 ? Number(answer.stdout.trim()) : 0
  }

  static async settledWidth(name, wanted) {
    let width = 0
    for (let attempt = 0; attempt < WindowGrantCheck.attempts && width !== wanted; attempt += 1) {
      await new Promise((done) => setTimeout(done, WindowGrantCheck.pause))
      width = WindowGrantCheck.width(name)
    }
    return width
  }

  static async steadyWidth(name) {
    await WindowGrantCheck.settledWidth(name, WindowGrantCheck.openedWidth)
    await new Promise((done) => setTimeout(done, WindowGrantCheck.steadyWait))
    return WindowGrantCheck.width(name)
  }

  static quit(name) {
    spawnSync('/usr/bin/osascript', ['-e', `tell application id "dev.tachyon.${name.toLowerCase()}" to quit`])
  }

  static async check(name, controls, wanted, label) {
    const app = WindowGrantCheck.project(name, controls)
    WindowGrantCheck.run('/usr/bin/open', ['-n', app])
    try {
      const granted = controls.length > 0
      const width = granted ? await WindowGrantCheck.settledWidth(name, wanted) : await WindowGrantCheck.steadyWidth(name)
      assert.equal(width, wanted, `${label}: the window is ${width} wide`)
      console.log(`ok - ${label} (${width} wide)`)
    } finally {
      WindowGrantCheck.quit(name)
    }
  }

  static async main() {
    try {
      WindowGrantCheck.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'CoreGraphics',
        join(WindowGrantCheck.repository, 'scripts/window-grant-macos-bounds.swift'), '-o', WindowGrantCheck.helper])
      await WindowGrantCheck.check(`TachyonGranted${process.pid}`, ['resize'], WindowGrantCheck.widenedWidth,
        'a granted resize from a service reaches the native window')
      await WindowGrantCheck.check(`TachyonUngranted${process.pid}`, [], WindowGrantCheck.openedWidth,
        'without the grant the native window keeps its size')
    } finally {
      rmSync(WindowGrantCheck.temporary, { recursive: true, force: true })
    }
  }
}

await WindowGrantCheck.main()
