#!/usr/bin/env node

import { CargoArtifacts } from './cargo-artifacts.mjs'
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

assert.equal(process.platform, 'darwin', 'The native application-state check needs macOS.')

class ApplicationStateCheck {
  static repository = resolve(dirname(fileURLToPath(import.meta.url)), '..')
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static temporary = mkdtempSync(join(tmpdir(), 'tachyon-application-state-'))
  static helper = join(ApplicationStateCheck.temporary, 'application-state-control')
  static log = join(ApplicationStateCheck.temporary, 'state.log')
  static applicationName = 'StateGate'
  static identifier = 'dev.tachyon.state-gate-macos'
  static listeners = ['page', 'rust', 'swift']
  static attempts = 60
  static pause = 250

  static files = {
    'client/pages/tac.html': '<state-panel hydrate="load" />\n<swift-panel hydrate="load" />\n',
    'client/components/state/panel/tac.html': '<main aria-label="State"><h1>State {stateWatch.state}</h1></main>\n',
    'client/components/swift/panel/tac.html': '<section aria-label="Swift state"><p>Swift {swiftState.state}</p></section>\n',
    'client/services/StateWatchService.js': `@Service
export default class StateWatchService {
  state = 'unknown'

  /** @param {ApplicationStateChanged} changed */
  @OnEvent({"replay":true})
  heard(changed) {
    this.state = changed.state
    void this.pageHeard(changed.state)
  }
}
`,
    'client/services/StateWatchService.rs': `#[Service]
#[derive(Default)]
struct StateWatchService {
    count: i64,
}

impl StateWatchService {
    pub fn page_heard(&self, state: String) -> i64 {
        Self::write(format!("page:{state}"));
        self.count
    }

    #[OnEvent({"replay":true})]
    fn native_heard(&mut self, changed: ApplicationStateChanged) {
        self.count += 1;
        Self::write(format!("rust:{}", changed.state));
    }

    fn write(line: String) {
        use std::io::Write;
        let path = std::env::var("STATE_GATE_LOG").unwrap_or_default();
        if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}
`,
    'client/services/SwiftStateService.swift': `import Foundation

@Service
final class SwiftStateService {
    var state: String = "unknown"

    @OnEvent({"replay":true})
    func applicationStateChanged(_ changed: ApplicationStateChanged) {
        state = changed.state
        SwiftStateService.write("swift:" + changed.state)
    }

    static func write(_ line: String) {
        guard let path = ProcessInfo.processInfo.environment["STATE_GATE_LOG"],
              let handle = FileHandle(forWritingAtPath: path) ?? SwiftStateService.created(path) else { return }
        handle.seekToEndOfFile()
        handle.write(Data((line + "\\n").utf8))
        handle.closeFile()
    }

    static func created(_ path: String) -> FileHandle? {
        FileManager.default.createFile(atPath: path, contents: nil)
        return FileHandle(forWritingAtPath: path)
    }
}
`,
  }

  static checksOff() {
    return Object.fromEntries(['names', 'visibility', 'cohesion'].map((name) => [name, { check: false }]))
  }

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

  static project() {
    const root = join(ApplicationStateCheck.temporary, 'project')
    ApplicationStateCheck.write(root, 'client/manifest.json', JSON.stringify({
      name: ApplicationStateCheck.applicationName, start_url: '/', tac: { application: { id: ApplicationStateCheck.identifier, version: '0.0.1' }, ...ApplicationStateCheck.checksOff() },
    }))
    for (const [relative, contents] of Object.entries(ApplicationStateCheck.files)) ApplicationStateCheck.write(root, relative, contents)
    ApplicationStateCheck.run(ApplicationStateCheck.binary, ['build', root, '--target', 'macos'])
    const app = join(root, 'dist/macos', `${ApplicationStateCheck.applicationName}.app`)
    assert.ok(existsSync(app), `${app} was not built`)
    return app
  }

  static heard() {
    return existsSync(ApplicationStateCheck.log) ? readFileSync(ApplicationStateCheck.log, 'utf8').split('\n').filter(Boolean) : []
  }

  static async waitFor(state, label) {
    const wanted = ApplicationStateCheck.listeners.map((listener) => `${listener}:${state}`)
    let lines = []
    for (let attempt = 0; attempt < ApplicationStateCheck.attempts; attempt += 1) {
      lines = ApplicationStateCheck.heard()
      if (wanted.every((line) => lines.includes(line))) break
      await new Promise((done) => setTimeout(done, ApplicationStateCheck.pause))
    }
    assert.deepEqual(wanted.filter((line) => !lines.includes(line)), [], `${label}: heard ${JSON.stringify(lines)}`)
    console.log(`ok - ${label}: ${wanted.join(', ')}`)
    writeFileSync(ApplicationStateCheck.log, '')
  }

  static control(action) {
    ApplicationStateCheck.run(ApplicationStateCheck.helper, [ApplicationStateCheck.identifier, action])
  }

  static async step(action, state, label) {
    ApplicationStateCheck.control(action)
    await ApplicationStateCheck.waitFor(state, label)
  }

  static quit() {
    spawnSync('/usr/bin/osascript', ['-e', `tell application id "${ApplicationStateCheck.identifier}" to quit`])
  }

  static async main() {
    try {
      ApplicationStateCheck.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit',
        join(ApplicationStateCheck.repository, 'scripts/application-state-macos-control.swift'), '-o', ApplicationStateCheck.helper])
      const app = ApplicationStateCheck.project()
      ApplicationStateCheck.run('/usr/bin/open', ['-n', '--env', `STATE_GATE_LOG=${ApplicationStateCheck.log}`, app])
      try {
        await ApplicationStateCheck.step('activate', 'foreground', 'the opened app is in the foreground')
        await ApplicationStateCheck.step('deactivate', 'background', 'another app became active')
        await ApplicationStateCheck.step('activate', 'foreground', 'the app became active again')
        await ApplicationStateCheck.step('hide', 'background', 'the app was hidden')
        await ApplicationStateCheck.step('unhide', 'foreground', 'the app was shown')
        await ApplicationStateCheck.step('minimize', 'background', 'the window was minimized')
        await ApplicationStateCheck.step('restore', 'foreground', 'the window was restored')
      } finally {
        ApplicationStateCheck.quit()
      }
      console.log('PASS: the macOS host sends background and foreground to the page and to linked Rust and Swift services')
    } finally {
      rmSync(ApplicationStateCheck.temporary, { recursive: true, force: true })
    }
  }
}

await ApplicationStateCheck.main()
