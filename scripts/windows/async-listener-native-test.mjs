#!/usr/bin/env node
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import path from 'node:path'
import { PageServicesFixture } from '../native/page-services-fixture.mjs'
import { WindowsGate } from './windows-gate.mjs'

class WindowsAsyncListenerTest {
  static languages = PageServicesFixture.list('TAC_WINDOWS_LISTENER_LANGUAGES', 'cs,rs')
  static names = { cs: 'CSharp', rs: 'Rust' }
  static companions = { cs: 'C#', rs: 'Rust' }
  static listenerPause = 400

  static models = {
    cs: 'public record Ping(int Step);\n',
    rs: 'pub struct Ping {\n    pub step: i64,\n}\n',
  }

  static services = {
    cs: `using Models;

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
    rs: `use crate::models::Ping;

#[derive(Default)]
#[Service]
struct HeardService {
    heard: String,
    listener_process: i64,
    application_event_publisher: ApplicationEventPublisher,
}

impl HeardService {
    pub fn send(&mut self, step: i64) {
        self.application_event_publisher.publish_event(Ping { step }, false);
    }

    #[OnEvent]
    async fn later(&mut self, value: Ping) {
        let done = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let mut started = false;
        std::future::poll_fn(|context| {
            if done.load(std::sync::atomic::Ordering::SeqCst) {
                return std::task::Poll::Ready(());
            }
            if !started {
                started = true;
                let waker = context.waker().clone();
                let flag = done.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_millis(${WindowsAsyncListenerTest.listenerPause}));
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    waker.wake();
                });
            }
            std::task::Poll::Pending
        })
        .await;
        self.heard.push_str(&format!("a{}", value.step));
        self.listener_process = i64::from(std::process::id());
    }

    #[OnEvent]
    fn now(&mut self, value: Ping) {
        self.heard.push_str(&format!("s{}", value.step));
    }
}
`,
  }

  static view = `<main aria-label="Async listener probe">
<p>Heard [{heard.heard}]</p>
<p>Listener process {heard.listenerProcess}</p>
<button on:click="heard.send(1)">Send one</button>
<button on:click="heard.send(2)">Send two</button>
</main>
`

  /** @param {string} root @param {string} language */
  static stage(root, language) {
    const application = `AsyncListener${this.names[language]}`
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const manifest = { name: application, start_url: '/', tac: { ...checks, application: { id: `dev.tachyon.async-listener-${language}`, version: '0.0.1' } } }
    WindowsGate.write(root, 'client/manifest.json', JSON.stringify(manifest, null, 2))
    WindowsGate.write(root, `client/models/Ping.${language}`, this.models[language])
    WindowsGate.write(root, `client/services/HeardService.${language}`, this.services[language])
    WindowsGate.write(root, 'client/components/async/probe/tac.html', this.view)
    WindowsGate.write(root, 'client/pages/tac.html', '<async-probe />\n')
    return application
  }

  /** @param {string} root @param {string} language */
  static assertLinked(root, language) {
    const host = JSON.parse(readFileSync(path.join(root, 'dist/windows/tachyon.host.json'), 'utf8'))
    const companion = this.companions[language]
    assert.ok(host.companions.some((item) => item.service === 'heard' && item.language === companion), `the ${companion} listener service is linked into the app: ${JSON.stringify(host.companions)}`)
  }

  /** @param {string} ty @param {string} language */
  static async check(ty, language) {
    const root = WindowsGate.fixture(`windows-async-listener-${language}-`)
    const application = this.stage(root, language)
    await WindowsGate.build(ty, root, 900_000)
    this.assertLinked(root, language)
    const app = WindowsGate.launch(WindowsGate.executable(root, application))
    try {
      const steps = [
        ['wait', ['Heard []', 'Send one']],
        ['click', 'Send one'],
        ['wait', ['Heard [a1s1]', `Listener process ${app.pid}`]],
        ['click', 'Send two'],
        ['wait', ['Heard [a1s1a2s2]']],
      ]
      const output = await WindowsGate.probeSteps(app.pid, steps, path.join(root, 'handshake'))
      console.log(`==> linked ${this.companions[language]} async listener\n${output.split(/\r?\n/).filter((line) => /^(OK|LOADED|NOTE|PASS)/.test(line)).join('\n')}`)
    } finally {
      await app.stop()
    }
  }

  static async main() {
    WindowsGate.requireDesktop()
    const ty = await WindowsGate.ty()
    for (const language of this.languages) {
      assert.ok(this.names[language], `unknown listener language ${language}; use cs or rs`)
      await this.check(ty, language)
    }
    const named = this.languages.map((language) => this.companions[language]).join(' and ')
    console.log(`PASS Windows: a linked ${named} async @OnEvent listener (${this.listenerPause} ms) finished inside the WebView2 app process before the next listener, for two events in a row.`)
  }
}

await WindowsAsyncListenerTest.main()
