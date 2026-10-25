#!/usr/bin/env node
import assert from 'node:assert/strict'
import { execFile, spawnSync } from 'node:child_process'
import { existsSync, mkdirSync, rmSync, writeFileSync } from 'node:fs'
import { homedir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { promisify } from 'node:util'
import { CargoArtifacts } from '../cargo-artifacts.mjs'

assert.equal(process.platform, 'darwin', 'The macOS geolocation check needs macOS.')

class MacOsGeolocationTest {
  static repository = fileURLToPath(new URL('../..', import.meta.url))
  static binary = process.env.TAC_BIN ?? CargoArtifacts.path('debug/ty')
  static root = path.join(process.env.TAC_PROBE_DIR ?? path.join(MacOsGeolocationTest.repository, 'target'), `macos-geolocation-${process.pid}`)
  static project = path.join(MacOsGeolocationTest.root, 'project')
  static probe = path.join(MacOsGeolocationTest.root, 'page-checks-macos-probe')
  static application = 'PlaceCheck'
  static identifier = `dev.tachyon.macos-geolocation.r${Date.now().toString(36)}`
  static reason = 'MA3 checks that a position reaches the page'
  static promptButtons = ['Allow While Using App', 'Allow Once', 'Allow']
  static waitMilliseconds = Number(process.env.TAC_MACOS_LOCATION_WAIT_SECONDS ?? 600) * 1000
  static allowItself = process.env.TAC_MACOS_LOCATION_ALLOW !== '0'

  static files = {
    'client/pages/tac.html': '<place-panel hydrate="load" />\n',
    'client/components/place/panel/tac.html': `<main>
<h1>Place check</h1>
<button type="button" on:click="placeCheck.locate()">Find my place</button>
<p>{placeCheck.place}</p>
<p>Accuracy {placeCheck.accuracy}</p>
<p>Problem {placeCheck.problem}</p>
</main>
`,
    'client/services/PlaceCheckService.js': `@Service
export default class PlaceCheckService {
  static LATITUDE_LIMIT = 90;
  static LONGITUDE_LIMIT = 180;
  static VERDICTS = ['Place out of range', 'Place found'];

  /** @type {TacGeolocation} */
  @Needs
  geolocation;
  place = 'No place yet';
  accuracy = 'none';
  problem = 'none';

  locate() {
    this.geolocation.currentPosition();
  }

  /** @param {PositionFound} found */
  @OnEvent
  found(found) {
    this.place = PlaceCheckService.VERDICTS[Number(this.#inRange(found))];
    this.accuracy = \`\${Math.round(found.accuracy)} m\`;
  }

  /** @param {PositionFound} found */
  #inRange(found) {
    return [Math.abs(found.latitude) <= PlaceCheckService.LATITUDE_LIMIT, Math.abs(found.longitude) <= PlaceCheckService.LONGITUDE_LIMIT, found.latitude !== 0 || found.longitude !== 0].every(Boolean);
  }

  /** @param {BrowserCallFailed} failed */
  @OnEvent
  failed(failed) {
    this.problem = failed.message;
  }
}
`,
  }

  static run(program, args, timeout = 900_000) {
    const result = spawnSync(program, args, { encoding: 'utf8', timeout, maxBuffer: 16 * 1024 * 1024 })
    assert.equal(result.status, 0, `${program} ${args.join(' ')} failed:\n${result.error ?? ''}${result.stdout}${result.stderr}`)
    return result.stdout.trim()
  }

  static write(name, text) {
    const target = path.join(this.project, name)
    mkdirSync(path.dirname(target), { recursive: true })
    writeFileSync(target, text)
  }

  static app() {
    return path.join(this.project, 'dist/macos', `${this.application}.app`)
  }

  static prepare() {
    for (const [name, text] of Object.entries(this.files)) this.write(name, text)
    this.write('client/manifest.json', JSON.stringify({
      name: this.application,
      start_url: '/',
      tac: {
        visibility: { check: false },
        browserApis: ['geolocation'],
        application: { id: this.identifier, version: '0.0.1', permissions: { geolocation: this.reason } },
      },
    }))
    this.run(this.binary, ['bundle', this.project, '--target', 'macos'])
    this.run('/usr/bin/xcrun', ['swiftc', '-warnings-as-errors', '-O', '-framework', 'AppKit',
      path.join(this.repository, 'scripts/native/page-checks-macos-probe.swift'), '-o', this.probe])
    const reason = this.run('/usr/bin/plutil', ['-extract', 'NSLocationWhenInUseUsageDescription', 'raw', path.join(this.app(), 'Contents/Info.plist')])
    assert.equal(reason, this.reason, 'Info.plist carries the manifest reason')
  }

  static async probeRun(args, timeout = 60_000) {
    return promisify(execFile)(this.probe, args, { timeout }).then(({ stdout }) => ({ ok: true, said: stdout.trim() }), (error) => ({ ok: false, said: `${error.stderr ?? error.message}`.trim() }))
  }

  static async locate() {
    const deadline = Date.now() + this.waitMilliseconds
    const tries = []
    let allowed = null
    console.log(this.allowItself
      ? '==> asking for a position; the gate presses Allow on the location prompt itself, or a person can click it'
      : '==> asking for a position; click Allow on the location prompt for PlaceCheck')
    while (Date.now() < deadline) {
      const pressed = await this.probeRun([this.identifier, 'press', 'Find my place'])
      assert.ok(pressed.ok, `the page has no Find my place button: ${pressed.said}`)
      if (this.allowItself && !allowed) {
        const answer = await this.probeRun(['-', 'allow', ...this.promptButtons])
        allowed = answer.ok ? answer.said : null
        tries.push(`allow: ${answer.said}`)
      }
      const found = await this.probeRun([this.identifier, 'wait', 'Place found'])
      if (found.ok) return { tries, allowed }
      tries.push(found.said.split('\n').at(-1))
    }
    assert.fail(`no position reached the page in ${this.waitMilliseconds / 1000} s: ${tries.join(' | ')}`)
  }

  static forgetWebsiteData() {
    const library = path.join(homedir(), 'Library')
    for (const folder of ['WebKit', 'Caches', 'HTTPStorages']) rmSync(path.join(library, folder, this.identifier), { recursive: true, force: true })
  }

  static async main() {
    const started = Date.now()
    mkdirSync(this.root, { recursive: true })
    let outcome
    try {
      this.prepare()
      this.run('/usr/bin/open', ['-n', this.app()])
      outcome = await this.locate()
    } finally {
      spawnSync('/usr/bin/pkill', ['-f', path.join(this.app(), 'Contents/MacOS')])
      this.forgetWebsiteData()
      if (process.env.TAC_KEEP_NATIVE_FIXTURE !== '1' && existsSync(this.root)) rmSync(this.root, { recursive: true, force: true })
    }
    const seconds = Math.round((Date.now() - started) / 1000)
    const answeredBy = outcome.allowed ? `the gate (${outcome.allowed})` : 'a person'
    console.log(`PASS: in the packaged macOS app (${this.identifier}), Info.plist carried the manifest location reason, the CoreLocation prompt was allowed by ${answeredBy}, and a position (latitude and longitude in range, not printed) reached the page through PositionFound (${seconds} s).`)
  }
}

await MacOsGeolocationTest.main()
