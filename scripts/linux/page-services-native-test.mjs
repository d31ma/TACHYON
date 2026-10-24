#!/usr/bin/env node
import assert from 'node:assert/strict'
import { cpSync, mkdirSync, mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { CargoArtifacts } from '../cargo-artifacts.mjs'
import { SupervisedCommand } from '../supervised-command.mjs'

class LinuxPageServicesTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = mkdtempSync(path.join(process.env.TAC_PROBE_DIR ?? tmpdir(), 'linux-page-services-'))
  static features = path.join(LinuxPageServicesTest.repository, 'website/content/features')
  static serviceLanguages = LinuxPageServicesTest.list('TAC_LINUX_SERVICE_LANGUAGES', 'go,cs,fs,swift,kt,java,dart,rb,lua,hpp,py')
  static windowCallers = LinuxPageServicesTest.list('TAC_LINUX_WINDOW_CALLERS', 'swift,cs,kt')
  static pickerCallers = LinuxPageServicesTest.list('TAC_LINUX_PICKER_CALLERS', 'swift,cs,kt')
  static pickerStatuses = { swift: ['read', 'cancelled'], kt: ['read', 'cancelled'], cs: ['Read', 'Cancelled'] }
  static tallySteps = [
    ['wait', ['0 in the basket', 'Add two']],
    ['click', 'Add two'],
    ['wait', ['2 in the basket', '2 items']],
  ]
  static reportSteps = [
    ['wait', ['Report', 'Rename']],
    ['click', 'Rename'],
    ['wait', ['Quarterly report']],
    ['click', 'Resize'],
    ['size', [1280, 800]],
  ]
  static screen = '1600x1200x24'

  /** @param {string} name @param {string} fallback */
  static list(name, fallback) {
    return (process.env[name] ?? fallback).split(',').map(item => item.trim()).filter(Boolean)
  }

  /** @param {string} command @param {string[]} args @param {import('../supervised-command.mjs').CommandOptions} options */
  static async run(command, args, options) {
    const result = await SupervisedCommand.run(command, args, { cwd: this.repository, ...options })
    assert.ok(result.status === 0 && result.cleanupConfirmed, `${command} ${args.join(' ')}\n${result.error?.message ?? ''}\n${result.stdout}${result.stderr}`)
    return result.stdout
  }

  /** @param {string} project @param {string} feature @param {string} name @param {string} id */
  static stage(project, feature, name, id) {
    const source = path.join(this.features, feature, 'client')
    for (const entry of ['pages', 'components']) cpSync(path.join(source, entry), path.join(project, 'client', entry), { recursive: true })
    const manifest = JSON.parse(readFileSync(path.join(source, 'manifest.json'), 'utf8'))
    manifest.tac = { ...manifest.tac, application: { ...manifest.tac?.application, id } }
    manifest.name ??= name
    writeFileSync(path.join(project, 'client/manifest.json'), JSON.stringify(manifest, null, 2))
    return manifest.name.replace(/[^A-Za-z0-9]/g, '')
  }

  /** @param {string} project @param {string} feature @param {string} file */
  static copyService(project, feature, file) {
    const target = path.join(project, 'client/services', path.basename(file))
    mkdirSync(path.dirname(target), { recursive: true })
    cpSync(path.join(this.features, feature, file), target)
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
    const application = this.stage(project, 'web-assembly-services', 'Tally', `dev.tachyon.tally-${language}`)
    this.copyService(project, 'web-assembly-services', `client/services/TallyService.${language}`)
    const started = Date.now()
    await this.build(project)
    const built = ((Date.now() - started) / 1000).toFixed(0)
    const output = await this.drive(project, application, this.tallySteps)
    console.log(`==> ${language} WebAssembly page service (built in ${built} s)\n${output}`)
  }

  /** @param {string} language */
  static async checkWindowCaller(language) {
    const project = path.join(this.root, `window-${language}`)
    const application = this.stage(project, 'window-controls', 'Catalog', `dev.tachyon.window-${language}`)
    this.copyService(project, 'window-controls', `client/services/ReportService.${language}`)
    await this.build(project)
    const output = await this.drive(project, application, this.reportSteps)
    console.log(`==> ${language} WebAssembly WindowControls caller\n${output}`)
  }

  /** @param {string} language */
  static async checkPickerCaller(language) {
    const project = path.join(this.root, `picker-${language}`)
    const application = this.stage(project, 'native-capabilities', 'Picker', `dev.tachyon.picker-${language}`)
    this.copyService(project, 'native-capabilities', `client/services/PickerService.${language}`)
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
