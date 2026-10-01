#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cpSync, mkdirSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from './cargo-artifacts.mjs'

class NativeWindowExamples {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static repo = fileURLToPath(new URL('../', import.meta.url))
  static feature = path.join(NativeWindowExamples.repo, 'website/content/features/window-controls')
  static cases = { rs: 'macos', swift: 'macos', cs: 'windows', kt: 'android' }

  static command(tool, args, cwd, environment = process.env) {
    const result = spawnSync(tool, args, { cwd, env: environment, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 })
    assert.equal(result.status, 0, result.error?.message ?? result.stderr ?? result.stdout)
    return result.stdout
  }

  static stage(root, extension, target) {
    const project = path.join(root, extension)
    const services = path.join(project, 'client/services')
    mkdirSync(services, { recursive: true })
    for (const entry of ['pages', 'components', 'manifest.json']) cpSync(path.join(NativeWindowExamples.feature, 'client', entry), path.join(project, 'client', entry), { recursive: true })
    cpSync(path.join(NativeWindowExamples.feature, 'native/services', `ReportService.${extension}`), path.join(services, `ReportService.${extension}`))
    NativeWindowExamples.command(NativeWindowExamples.binary, ['bundle', project, '--target', target, '--native', '--no-incremental'], NativeWindowExamples.repo)
    console.log(`${extension}: ordinary CLI staged native website source for ${target}`)
    return project
  }

  static run() {
    assert.equal(process.platform, 'darwin', 'native window example matrix requires macOS')
    const root = path.resolve(process.env.TAC_NATIVE_WINDOW_OUT ?? mkdtempSync(path.join(tmpdir(), 'tachyon-native-window-examples-')))
    mkdirSync(root, { recursive: true })
    console.log(`retained native example artifacts: ${root}`)
    for (const [extension, target] of Object.entries(NativeWindowExamples.cases)) NativeWindowExamples.stage(root, extension, target)
    const test = 'native::native_window_example_tests::staged_native_window_examples_use_supervised_tools'
    const output = NativeWindowExamples.command('cargo', ['test', '-p', 'tachyon-core', '--lib', '--all-features', test, '--locked', '--', '--exact', '--ignored', '--nocapture', '--test-threads=1'], NativeWindowExamples.repo, { ...process.env, TAC_NATIVE_WINDOW_OUT: root })
    assert.match(output, /test result: ok\. 1 passed; 0 failed;/, 'exact supervised verification test must execute')
    console.log(output.trim())
  }
}

NativeWindowExamples.run()
