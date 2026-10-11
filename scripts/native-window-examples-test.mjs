#!/usr/bin/env node
import assert from 'node:assert/strict'
import { cpSync, mkdirSync, mkdtempSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from './cargo-artifacts.mjs'
import { SupervisedCommand } from './supervised-command.mjs'

class NativeWindowExamples {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static repo = fileURLToPath(new URL('../', import.meta.url))
  static testRepo = process.env.TAC_NATIVE_WINDOW_TEST_REPO ?? NativeWindowExamples.repo
  static feature = path.join(NativeWindowExamples.repo, 'website/content/features/window-controls')
  static cases = { rs: 'macos', swift: 'macos', cs: 'windows', kt: 'android' }
  // Rust has no native alternative: the page's client service is the one a native app links (native/targets.txt).
  static serviceFolders = { rs: 'client/services' }
  // dotnet run otherwise leaves its compiler and MSBuild servers running after the owned command family ends.
  static noBuildServers = { UseSharedCompilation: 'false', MSBUILDDISABLENODEREUSE: '1', DOTNET_CLI_USE_MSBUILD_SERVER: '0' }

  static async command(tool, args, cwd, environment = process.env) {
    const timeoutMilliseconds = Number(process.env.TAC_NATIVE_WINDOW_TIMEOUT_MS ?? SupervisedCommand.defaultTimeoutMilliseconds)
    const result = await SupervisedCommand.run(tool, args, { cwd, env: environment, timeoutMilliseconds, failOnOutputLimit: true })
    assert.equal(result.error, undefined, `${result.error?.message ?? ''}\n${result.stderr}\n${result.stdout}`)
    assert.equal(result.cleanupConfirmed, true, 'native command family cleanup must be confirmed')
    assert.equal(result.status, 0, result.stderr || result.stdout)
    return result.stdout
  }

  static async stage(root, extension, target) {
    const project = path.join(root, extension)
    const services = path.join(project, 'client/services')
    mkdirSync(services, { recursive: true })
    for (const entry of ['pages', 'components', 'manifest.json']) cpSync(path.join(NativeWindowExamples.feature, 'client', entry), path.join(project, 'client', entry), { recursive: true })
    const folder = NativeWindowExamples.serviceFolders[extension] ?? 'native/services'
    cpSync(path.join(NativeWindowExamples.feature, folder, `ReportService.${extension}`), path.join(services, `ReportService.${extension}`))
    await NativeWindowExamples.command(NativeWindowExamples.binary, ['bundle', project, '--target', target, '--native', '--no-incremental'], NativeWindowExamples.repo)
    console.log(`${extension}: ordinary CLI staged native website source for ${target}`)
    return project
  }

  static async run() {
    assert.equal(process.platform, 'darwin', 'native window example matrix requires macOS')
    const root = path.resolve(process.env.TAC_NATIVE_WINDOW_OUT ?? mkdtempSync(path.join(tmpdir(), 'tachyon-native-window-examples-')))
    mkdirSync(root, { recursive: true })
    console.log(`retained native example artifacts: ${root}`)
    for (const [extension, target] of Object.entries(NativeWindowExamples.cases)) await NativeWindowExamples.stage(root, extension, target)
    const test = 'native::native_window_example_tests::staged_native_window_examples_use_supervised_tools'
    const output = await NativeWindowExamples.command('cargo', ['test', '-p', 'tachyon-core', '--lib', '--all-features', test, '--locked', '--', '--exact', '--ignored', '--nocapture', '--test-threads=1'], NativeWindowExamples.testRepo, { ...process.env, ...NativeWindowExamples.noBuildServers, TAC_NATIVE_WINDOW_OUT: root })
    assert.match(output, /test result: ok\. 1 passed; 0 failed;/, 'exact supervised verification test must execute')
    console.log(output.trim())
  }
}

await NativeWindowExamples.run()
