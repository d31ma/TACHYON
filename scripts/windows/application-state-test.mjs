#!/usr/bin/env node
import assert from 'node:assert/strict'
import { rmSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { setTimeout as sleep } from 'node:timers/promises'
import { WindowsGate } from './windows-gate.mjs'

class WindowsApplicationStateTest {
  static fixture = WindowsGate.fixture('windows-application-state-')
  static handshake = path.join(WindowsApplicationStateTest.fixture, 'handshake')
  static stateLog = path.join(WindowsApplicationStateTest.fixture, 'state.log')
  static application = 'StateGate'
  static applicationId = 'dev.tachyon.state-gate'
  static waitMilliseconds = 20_000
  static steps = [
    ['wait', ['State foreground']],
    ['activate', 'State Gate'],
    ['ask', 'first-foreground'],
    ['deactivate', 'Other app'],
    ['ask', 'background'],
    ['wait', ['State background']],
    ['activate', 'State Gate'],
    ['ask', 'foreground'],
    ['wait', ['State foreground']],
    ['minimize', 'State Gate'],
    ['ask', 'minimized'],
  ]
  static files = {
    'client/manifest.json': JSON.stringify({
      name: 'State Gate',
      start_url: '/',
      tac: { application: { id: 'dev.tachyon.state-gate', version: '0.0.1' } },
    }),
    'client/pages/tac.html': '<state-panel />\n',
    'client/components/state/panel/tac.html': '<main aria-label="State"><h1>State {stateWatch.state}</h1></main>\n',
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
        Self::write(format!("native:{}", changed.state));
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
  }

  /** @param {string[]} lines */
  static async waitForLines(lines) {
    const deadline = Date.now() + this.waitMilliseconds
    let seen = []
    while (Date.now() < deadline) {
      seen = WindowsGate.readLines(this.stateLog)
      if (lines.every(line => seen.includes(line))) {
        console.log(`OK: the state log has ${lines.join(', ')}`)
        writeFileSync(this.stateLog, '')
        return
      }
      await sleep(250)
    }
    throw new Error(`The state log lacks ${lines.filter(line => !seen.includes(line)).join(', ')}; it has ${JSON.stringify(seen)}`)
  }

  static checkLifecycleLog(log) {
    const events = WindowsGate.readLines(log)
    for (const event of ['application.background', 'application.foreground']) {
      assert.ok(events.some(line => line.includes(`"event":"${event}"`)), `the host lifecycle log ${log} has no ${event}: ${events.join(' | ')}`)
    }
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    for (const [name, text] of Object.entries(this.files)) WindowsGate.write(this.fixture, name, text)
    console.log('==> building the Windows host with a page service and a linked Rust service')
    await WindowsGate.build(ty, this.fixture)
    const executable = WindowsGate.executable(this.fixture, this.application)
    const log = WindowsGate.lifecycleLog(this.applicationId)
    rmSync(log, { force: true })
    console.log('==> sending the app to the background and back, then minimizing it')
    const app = WindowsGate.launch(executable, { ...process.env, STATE_GATE_LOG: this.stateLog })
    try {
      const probe = WindowsGate.startProbe(app.pid, this.steps, this.handshake)
      await WindowsGate.answerAll(probe, this.handshake, {
        'first-foreground': () => this.waitForLines(['native:foreground']),
        background: () => this.waitForLines(['native:background', 'page:background']),
        foreground: () => this.waitForLines(['native:foreground', 'page:foreground']),
        minimized: () => this.waitForLines(['native:background']),
      })
    } finally {
      await app.stop()
    }
    this.checkLifecycleLog(log)
    console.log('PASS: the Windows host (WM_ACTIVATEAPP and a minimize) sends background and foreground to the WebView2 page and to a linked Rust service')
  }
}

await WindowsApplicationStateTest.main()
