#!/usr/bin/env node
import assert from 'node:assert/strict'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'
import { PageServicesFixture } from '../native/page-services-fixture.mjs'

class LinuxPageServicesTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-page-services-'))
  static serviceLanguages = PageServicesFixture.list('TAC_LINUX_SERVICE_LANGUAGES', 'go,cs,fs,swift,kt,java,dart,rb,lua,hpp,py')
  static windowCallers = PageServicesFixture.list('TAC_LINUX_WINDOW_CALLERS', 'swift,cs,kt')
  static pickerCallers = PageServicesFixture.list('TAC_LINUX_PICKER_CALLERS', 'swift,cs,kt')
  static pickerStatuses = { swift: ['read', 'cancelled'], kt: ['read', 'cancelled'], cs: ['Read', 'Cancelled'] }
  static tallySteps = PageServicesFixture.tallySteps
  static reportSteps = PageServicesFixture.reportSteps
  static screen = '1600x1200x24'

  /** @param {string} command @param {string[]} args @param {import('../supervised-command.mjs').CommandOptions} options */
  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  /** @param {string} project @param {string} application @param {Array<[string, unknown]>} steps @param {Record<string, string>} settings */
  static async drive(project, application, steps, settings = {}) {
    const app = path.join(project, 'dist/linux', application, 'bin', application)
    const script = `set -euo pipefail
if [[ -n "\${WINDOW_MANAGER:-}" ]]; then xfwm4 --compositor=off --sm-client-disable >/dev/null 2>&1 & sleep 1; fi
"$APP" &
host=$!
trap 'kill "$host" 2>/dev/null || true; wait "$host" 2>/dev/null || true' EXIT
timeout 300s python3 "$PROBE" "$STEPS"
tree=$host
pending=$host
while [[ -n "$pending" ]]; do
  pending=$(for parent in $pending; do pgrep -P "$parent" || true; done | tr '\\n' ' ')
  tree="$tree $pending"
done
echo "MEMORY: $(ps -o rss= -p "$(echo $tree | tr ' ' ',')" | awk '{ total += $1 } END { printf "%.0f", total / 1024 }') MiB resident in the app and its web processes"`
    const env = {
      ...process.env,
      APP: app,
      PROBE: path.join(this.repository, 'scripts/linux/page-steps-probe.py'),
      STEPS: JSON.stringify(steps),
      TAC_PROBE_APPLICATION: application,
      XDG_STATE_HOME: path.join(project, 'state'),
      XDG_DATA_HOME: path.join(project, 'data'),
      XDG_CACHE_HOME: path.join(project, 'cache'),
      GDK_BACKEND: 'x11',
      GTK_A11Y: 'atspi',
      LIBGL_ALWAYS_SOFTWARE: '1',
      WEBKIT_DMABUF_RENDERER_FORCE_SHM: '1',
      ...settings,
    }
    const output = await this.run('xvfb-run', ['--auto-servernum', `--server-args=-screen 0 ${this.screen}`, 'dbus-run-session', '--', 'bash', '-c', script], { env, timeoutMilliseconds: 360000 })
    return output.split('\n').filter(line => /^(OK|LOADED|MEMORY|PASS)/.test(line)).join('\n')
  }

  /** @param {string} project */
  static async build(project) {
    await this.run(this.binary, ['build', project, '--target', 'linux'], { timeoutMilliseconds: 900000 })
  }

  /** @param {string} language */
  static async checkService(language) {
    const project = path.join(this.root, `tally-${language}`)
    const application = PageServicesFixture.stageTally(project, language)
    const started = Date.now()
    await this.build(project)
    const built = ((Date.now() - started) / 1000).toFixed(0)
    const output = await this.drive(project, application, this.tallySteps)
    console.log(`==> ${language} WebAssembly page service (built in ${built} s)\n${output}`)
  }

  /** @param {string} language */
  static async checkWindowCaller(language) {
    const project = path.join(this.root, `window-${language}`)
    const application = PageServicesFixture.stageWindowCaller(project, language)
    await this.build(project)
    const output = await this.drive(project, application, this.reportSteps)
    console.log(`==> ${language} WebAssembly WindowControls caller\n${output}`)
  }

  /** @param {string} language */
  static async checkPickerCaller(language) {
    const project = path.join(this.root, `picker-${language}`)
    const application = PageServicesFixture.stage(project, 'native-capabilities', 'Picker', `dev.tachyon.picker-${language}`)
    PageServicesFixture.copyService(project, 'native-capabilities', `client/services/PickerService.${language}`)
    const notes = path.join(project, 'notes.txt')
    writeFileSync(notes, 'Quarterly notes\n')
    await this.build(project)
    const [read, cancelled] = this.pickerStatuses[language]
    const steps = [
      ['wait', ['Choose a text file', 'Ready']],
      ['pick', { control: 'Choose file', keys: ['ctrl+l', { type: notes }, 'Return'] }],
      ['wait', [read]],
      ['pick', { control: 'Choose file', keys: ['Escape'] }],
      ['wait', [cancelled]],
    ]
    const output = await this.drive(project, application, steps, { WINDOW_MANAGER: '1' })
    console.log(`==> ${language} WebAssembly FilePicker caller\n${output}`)
  }

  static async main() {
    for (const language of this.serviceLanguages) await this.checkService(language)
    for (const language of this.windowCallers) await this.checkWindowCaller(language)
    for (const language of this.pickerCallers) await this.checkPickerCaller(language)
    console.log(`PASS: WebAssembly page services (${this.serviceLanguages.join(', ')}) answered in the packaged Linux app, WindowControls callers (${this.windowCallers.join(', ')}) renamed and resized its window, and FilePicker callers (${this.pickerCallers.join(', ')}) chose, read and cancelled through the file dialog.`)
  }
}

await LinuxPageServicesTest.main()
