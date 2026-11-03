#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'

class LinuxAsyncListenerTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-async-listener-'))
  static application = 'AsyncListenerRust'
  static listenerPause = 400
  static screen = '1280x900x24'
  static steps = [
    ['wait', ['[]', 'Send one', 'Send two']],
    ['click', 'Send one'],
    ['wait', ['[a1s1]']],
    ['click', 'Send two'],
    ['wait', ['[a1s1a2s2]']],
  ]

  static model = 'pub struct Ping {\n    pub step: i64,\n}\n'

  static service = `use crate::models::Ping;

#[derive(Default)]
#[Service]
struct HeardService {
    heard: String,
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
                    std::thread::sleep(std::time::Duration::from_millis(${LinuxAsyncListenerTest.listenerPause}));
                    flag.store(true, std::sync::atomic::Ordering::SeqCst);
                    waker.wake();
                });
            }
            std::task::Poll::Pending
        })
        .await;
        self.heard.push_str(&format!("a{}", value.step));
    }

    #[OnEvent]
    fn now(&mut self, value: Ping) {
        self.heard.push_str(&format!("s{}", value.step));
    }
}
`

  static view = `<main aria-label="Async listener probe">
<p>[{heard.heard}]</p>
<button on:click="heard.send(1)">Send one</button>
<button on:click="heard.send(2)">Send two</button>
</main>
`

  /** @param {string} project @param {string} name @param {string} contents */
  static write(project, name, contents) {
    const file = path.join(project, name)
    mkdirSync(path.dirname(file), { recursive: true })
    writeFileSync(file, contents)
  }

  /** @param {string} project */
  static stage(project) {
    const checks = Object.fromEntries(['names', 'constants', 'visibility', 'duplication', 'cohesion'].map((name) => [name, { check: false }]))
    const manifest = { name: this.application, start_url: '/', tac: { ...checks, application: { id: 'dev.tachyon.async-listener-rust', version: '0.0.1' } } }
    this.write(project, 'client/manifest.json', JSON.stringify(manifest, null, 2))
    this.write(project, 'client/models/Ping.rs', this.model)
    this.write(project, 'client/services/HeardService.rs', this.service)
    this.write(project, 'client/components/async/probe/tac.html', this.view)
    this.write(project, 'client/pages/tac.html', '<async-probe />\n')
  }

  /** @param {string} command @param {string[]} args @param {import('../supervised-command.mjs').CommandOptions} options */
  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  /** @param {string} output */
  static assertLinked(output) {
    const host = JSON.parse(readFileSync(path.join(output, 'tachyon.host.json'), 'utf8'))
    assert.ok(host.companions.some((item) => item.service === 'heard' && item.language === 'Rust'), `the Rust listener service is linked into the app: ${JSON.stringify(host.companions)}`)
    const modules = readdirSync(output, { recursive: true }).filter((name) => String(name).endsWith('.wasm'))
    assert.deepEqual(modules, [], 'the linked service is not also shipped as WebAssembly')
  }

  /** @param {string} project */
  static async drive(project) {
    const app = path.join(project, 'dist/linux', this.application, 'bin', this.application)
    const script = `set -euo pipefail
"$APP" &
host=$!
trap 'kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true' EXIT
timeout 300s python3 "$PROBE" "$STEPS"`
    const env = {
      ...process.env,
      APP: app,
      PROBE: path.join(this.repository, 'scripts/linux/page-steps-probe.py'),
      STEPS: JSON.stringify(this.steps),
      TAC_PROBE_APPLICATION: this.application,
      XDG_STATE_HOME: path.join(project, 'state'),
      XDG_DATA_HOME: path.join(project, 'data'),
      XDG_CACHE_HOME: path.join(project, 'cache'),
      GDK_BACKEND: 'x11',
      GTK_A11Y: 'atspi',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
    }
    const output = await this.run('xvfb-run', ['--auto-servernum', `--server-args=-screen 0 ${this.screen}`, 'dbus-run-session', '--', 'bash', '-c', script], { env, timeoutMilliseconds: 360000 })
    return output.split('\n').filter((line) => /^(OK|LOADED|PASS)/.test(line)).join('\n')
  }

  static async main() {
    const project = path.join(this.root, 'project')
    this.stage(project)
    await this.run(this.binary, ['build', project, '--target', 'linux'], { timeoutMilliseconds: 900000 })
    this.assertLinked(path.join(project, 'dist/linux'))
    console.log(await this.drive(project))
    console.log('PASS: a linked Rust async listener finished in the Linux app before the plain one, and the page showed both')
  }
}

await LinuxAsyncListenerTest.main()
