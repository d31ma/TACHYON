#!/usr/bin/env node
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from './cargo-artifacts.mjs'

class NativeWindowExamples {
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static repo = fileURLToPath(new URL('../', import.meta.url))
  static feature = path.join(NativeWindowExamples.repo, 'website/content/features/window-controls')
  static cases = { rs: 'macos', swift: 'macos', cs: 'windows', kt: 'android' }

  static command(tool, args, cwd) {
    const result = spawnSync(tool, args, { cwd, encoding: 'utf8', timeout: 180000, killSignal: 'SIGKILL', maxBuffer: 4 * 1024 * 1024 })
    assert.equal(result.status, 0, result.error?.message ?? result.stderr ?? result.stdout)
    return result.stdout
  }

  static find(directory, name) {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const current = path.join(directory, entry.name)
      if (entry.isFile() && entry.name === name) return current
      if (entry.isDirectory()) {
        const found = NativeWindowExamples.find(current, name)
        if (found) return found
      }
    }
    return null
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

  static macos(project, extension) {
    const host = NativeWindowExamples.find(path.join(project, 'dist'), 'TachyonHost.swift')
    assert.ok(host, 'generated macOS host exists')
    const source = readFileSync(host, 'utf8')
    const end = source.lastIndexOf('@main')
    assert.ok(end > 0)
    const rust = extension === 'rs'
    const registration = rust ? 'tacRustNativeSetCapabilityAdapter(tacRustCapabilityCall, tacRustCapabilityFree)' : 'NativeCapabilities.call = { capability, payload in MainActor.assumeIsolated { NativeBridge.shared.handle(capability, payload) } }'
    const check = readFileSync(new URL('./native/NativeWindowExampleCheck.swift', import.meta.url), 'utf8').replaceAll('__CALL__', rust ? 'tacRustInvoke' : 'tacNativeInvoke').replace('__REGISTER__', registration)
    const checked = path.join(project, 'WindowCheck.swift')
    writeFileSync(checked, source.slice(0, end) + check)
    const executable = path.join(project, 'window-check')
    const args = ['swiftc', '-parse-as-library', '-framework', 'AppKit', '-framework', 'WebKit', checked]
    if (rust) {
      const companion = NativeWindowExamples.find(path.join(project, 'dist'), 'companion.rs')
      assert.ok(companion)
      const library = path.join(project, 'report.a')
      NativeWindowExamples.command('rustc', ['--edition', '2024', '--crate-type', 'staticlib', '-O', companion, '-o', library], project)
      args.push(library)
    } else {
      const companion = NativeWindowExamples.find(path.join(project, 'dist'), 'TachyonCompanion.swift')
      assert.ok(companion)
      args.push(companion)
    }
    args.push('-o', executable)
    NativeWindowExamples.command('/usr/bin/xcrun', args, project)
    console.log(`${extension}: ${NativeWindowExamples.command(executable, [], project).trim()}`)
  }

  static csharp(project) {
    const companion = NativeWindowExamples.find(path.join(project, 'dist'), 'TachyonCompanion.cs')
    assert.ok(companion)
    const check = path.join(project, 'managed-check')
    mkdirSync(check)
    writeFileSync(path.join(check, 'Program.cs'), readFileSync(companion, 'utf8') + readFileSync(new URL('./native/WindowExampleCheck.cs', import.meta.url), 'utf8'))
    writeFileSync(path.join(check, 'check.csproj'), '<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net9.0</TargetFramework><AllowUnsafeBlocks>true</AllowUnsafeBlocks><Nullable>disable</Nullable><RestoreIgnoreFailedSources>true</RestoreIgnoreFailedSources></PropertyGroup></Project>')
    console.log(NativeWindowExamples.command('dotnet', ['run', '--project', path.join(check, 'check.csproj')], project).trim())
  }

  static kotlin(project) {
    const companion = NativeWindowExamples.find(path.join(project, 'dist'), 'TachyonCompanion.kt')
    const android = process.env.ANDROID_HOME ?? process.env.ANDROID_SDK_ROOT
    assert.ok(companion)
    assert.ok(android, 'Android SDK required for the qualified Kotlin check')
    const sdk = path.join(android, 'platforms/android-35/android.jar')
    assert.ok(existsSync(sdk))
    NativeWindowExamples.command('kotlinc', [companion, '-classpath', sdk, '-d', path.join(project, 'report.jar')], project)
    console.log('kt: complete generated ReportService companion typechecked against Android API35; no Android OS runtime claim')
  }

  static run() {
    const root = process.env.TAC_NATIVE_WINDOW_OUT ?? mkdtempSync(path.join(tmpdir(), 'tachyon-native-window-examples-'))
    mkdirSync(root, { recursive: true })
    console.log(`retained native example artifacts: ${root}`)
    for (const [extension, target] of Object.entries(NativeWindowExamples.cases)) {
      const project = NativeWindowExamples.stage(root, extension, target)
      if (extension === 'rs' || extension === 'swift') NativeWindowExamples.macos(project, extension)
      if (extension === 'cs') NativeWindowExamples.csharp(project)
      if (extension === 'kt') NativeWindowExamples.kotlin(project)
    }
  }
}

NativeWindowExamples.run()
