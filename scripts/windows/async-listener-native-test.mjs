#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { WindowsGate } from './windows-gate.mjs'

class WindowsAsyncListenerTest {
  static root = WindowsGate.fixture('windows-async-listener-')
  static application = 'AsyncListenerCSharp'
  static listenerPause = 400

  static files = {
    'client/models/Ping.cs': 'public record Ping(int Step);\n',
    'client/services/HeardService.cs': `using Models;

[Service]
public sealed class HeardService
{
    private readonly ApplicationEventPublisher applicationEventPublisher;

    public string Heard = "";
    public int ListenerProcess;

    public void Send(int step)
    {
        applicationEventPublisher.PublishEvent(new Ping(step));
    }

    [OnEvent]
    public async System.Threading.Tasks.Task Later(Ping ping)
    {
        await System.Threading.Tasks.Task.Delay(${WindowsAsyncListenerTest.listenerPause});
        Heard += "a" + ping.Step;
        ListenerProcess = System.Environment.ProcessId;
    }

    [OnEvent]
    public void Now(Ping ping)
    {
        Heard += "s" + ping.Step;
    }
}
`,
    'client/components/async/probe/tac.html': `<main aria-label="Async listener probe">
<p>Heard [{heard.heard}]</p>
<p>Listener process {heard.listenerProcess}</p>
<button on:click="heard.send(1)">Send one</button>
<button on:click="heard.send(2)">Send two</button>
</main>
`,
    'client/pages/tac.html': '<async-probe />\n',
  }

  static stage() {
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const manifest = { name: this.application, start_url: '/', tac: { ...checks, application: { id: 'dev.tachyon.async-listener-cs', version: '0.0.1' } } }
    WindowsGate.write(this.root, 'client/manifest.json', JSON.stringify(manifest, null, 2))
    for (const [name, text] of Object.entries(this.files)) WindowsGate.write(this.root, name, text)
  }

  static assertLinked() {
    const host = JSON.parse(readFileSync(path.join(this.root, 'dist/windows/tachyon.host.json'), 'utf8'))
    assert.ok(host.companions.some((item) => item.service === 'heard' && item.language === 'C#'), `the C# listener service is linked into the app: ${JSON.stringify(host.companions)}`)
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    this.stage()
    await WindowsGate.build(ty, this.root, 900_000)
    this.assertLinked()
    const app = WindowsGate.launch(WindowsGate.executable(this.root, this.application))
    try {
      const steps = [
        ['wait', ['Heard []', 'Send one']],
        ['click', 'Send one'],
        ['wait', ['Heard [a1s1]', `Listener process ${app.pid}`]],
        ['click', 'Send two'],
        ['wait', ['Heard [a1s1a2s2]']],
      ]
      const output = await WindowsGate.probeSteps(app.pid, steps, path.join(this.root, 'handshake'))
      console.log(output.split(/\r?\n/).filter((line) => /^(OK|LOADED|NOTE|PASS)/.test(line)).join('\n'))
    } finally {
      await app.stop()
    }
    console.log(`PASS Windows: a linked C# async Task @OnEvent listener (${this.listenerPause} ms) finished inside the WebView2 app process before the next listener, for two events in a row.`)
  }
}

await WindowsAsyncListenerTest.main()
