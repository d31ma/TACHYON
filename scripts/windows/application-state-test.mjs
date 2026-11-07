#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFileSync, rmSync, writeFileSync } from 'node:fs'
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
    'client/components/state/panel/tac.html': '<main aria-label="State"><h1>State {stateWatch.state}</h1><p>CSharp {stateEcho.state}</p></main>\n',
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
    'client/services/StateEchoService.js': `@Service
export default class StateEchoService {
  state = 'unknown'

  /** @param {ApplicationStateChanged} changed */
  @OnEvent({"replay":true})
  heard(changed) {
    this.state = changed.state
    void this.pageHeard(changed.state)
  }
}
`,
    'client/services/StateEchoService.cs': `[Service]
public sealed class StateEchoService
{
    private int _count;

    public int PageHeard(string state)
    {
        Write("csharp-page:" + state);
        return _count;
    }

    [OnEvent({"replay":true})]
    public void NativeHeard(ApplicationStateChanged changed)
    {
        _count += 1;
        Write("csharp:" + changed.State);
    }

    private static void Write(string line)
    {
        var path = System.Environment.GetEnvironmentVariable("STATE_GATE_LOG");
        if (string.IsNullOrEmpty(path)) return;
        using var file = new System.IO.FileStream(path, System.IO.FileMode.Append, System.IO.FileAccess.Write, System.IO.FileShare.ReadWrite | System.IO.FileShare.Delete);
        using var writer = new System.IO.StreamWriter(file);
        writer.WriteLine(line);
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

  static assertLinked() {
    const host = JSON.parse(readFileSync(path.join(this.fixture, 'dist/windows/tachyon.host.json'), 'utf8'))
    for (const [service, language] of [['stateWatch', 'Rust'], ['stateEcho', 'C#']]) {
      assert.ok(host.companions.some(item => item.service === service && item.language === language), `the ${language} ${service} service is linked into the app: ${JSON.stringify(host.companions)}`)
    }
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
    console.log('==> building the Windows host with a page service, a linked Rust service and a linked C# service')
    await WindowsGate.build(ty, this.fixture)
    this.assertLinked()
    const executable = WindowsGate.executable(this.fixture, this.application)
    const log = WindowsGate.lifecycleLog(this.applicationId)
    rmSync(log, { force: true })
    console.log('==> sending the app to the background and back, then minimizing it')
    const app = WindowsGate.launch(executable, { ...process.env, STATE_GATE_LOG: this.stateLog })
    try {
      const probe = WindowsGate.startProbe(app.pid, this.steps, this.handshake)
      await WindowsGate.answerAll(probe, this.handshake, {
        'first-foreground': () => this.waitForLines(['native:foreground', 'csharp:foreground']),
        background: () => this.waitForLines(['native:background', 'page:background', 'csharp:background', 'csharp-page:background']),
        foreground: () => this.waitForLines(['native:foreground', 'page:foreground', 'csharp:foreground', 'csharp-page:foreground']),
        minimized: () => this.waitForLines(['native:background', 'csharp:background']),
      })
    } finally {
      await app.stop()
    }
    this.checkLifecycleLog(log)
    console.log('PASS: the Windows host (WM_ACTIVATEAPP and a minimize) sends background and foreground to the WebView2 page, to a linked Rust service and to a linked C# service')
  }
}

await WindowsApplicationStateTest.main()
