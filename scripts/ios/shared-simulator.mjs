import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { homedir } from 'node:os'
import path from 'node:path'

/**
 * @typedef {Object} SharedDevice
 * @property {string} device
 * @property {{ identifier: string, version: string }} runtime
 * @property {{ identifier: string, name: string, productFamily: string }} type
 */

export class SharedSimulator {
  static DEFAULT_SET = path.join(homedir(), 'Library/Developer/CoreSimulator/Devices')
  static VARIABLES = { iPhone: 'TAC_IOS_DEVICE', iPad: 'TAC_IOS_TABLET_DEVICE' }
  static LIST_MILLISECONDS = 120000
  static CLEANUP_MILLISECONDS = 60000
  static FAMILIES_VARIABLE = 'TAC_IOS_FAMILIES'

  /** @param {string} family @returns {string} */
  static udid(family) {
    return process.env[SharedSimulator.VARIABLES[family]] ?? ''
  }

  /** @returns {string[]} */
  static families() {
    const known = Object.keys(SharedSimulator.VARIABLES)
    const wanted = SharedSimulator.wantedFamilies(known)
    const unknown = wanted.filter((family) => !known.includes(family))
    assert.equal(unknown.length, 0, `${SharedSimulator.FAMILIES_VARIABLE} names an unknown iOS family: ${unknown.join(', ')}. Use iPhone, iPad, or iPhone,iPad.`)
    assert.ok(wanted.length > 0, `${SharedSimulator.FAMILIES_VARIABLE} names no iOS family. Use iPhone, iPad, or iPhone,iPad.`)
    return known.filter((family) => wanted.includes(family))
  }

  /** @param {string[]} known @returns {string[]} */
  static wantedFamilies(known) {
    const text = process.env[SharedSimulator.FAMILIES_VARIABLE] ?? known.join(',')
    return text.split(',').map((family) => family.trim()).filter((family) => family !== '')
  }

  /** @returns {string[]} */
  static sharedDevices() {
    return Object.keys(SharedSimulator.VARIABLES).map((family) => SharedSimulator.udid(family)).filter((udid) => udid !== '')
  }

  /** @returns {boolean} */
  static chosen() {
    return SharedSimulator.sharedDevices().length > 0
  }

  /** @param {string} device @returns {boolean} */
  static isShared(device) {
    return SharedSimulator.sharedDevices().includes(device)
  }

  /** @param {string} privateSet @returns {string} */
  static deviceSet(privateSet) {
    return process.env.TAC_IOS_DEVICE_SET ?? SharedSimulator.setWhenNotNamed(privateSet)
  }

  /** @param {string} privateSet @returns {string} */
  static setWhenNotNamed(privateSet) {
    if (SharedSimulator.chosen()) return SharedSimulator.DEFAULT_SET
    return privateSet
  }

  /** @param {{ identifier: string, version: string }} runtime @returns {boolean} */
  static wantedRuntime(runtime) {
    const wanted = process.env.TAC_IOS_RUNTIME ?? ''
    return `${runtime.version}.`.startsWith(`${wanted}.`) || runtime.identifier.includes(wanted)
  }

  /** @param {string} deviceSet @param {string[]} args @returns {any} */
  static list(deviceSet, args) {
    const result = spawnSync('/usr/bin/xcrun', ['simctl', '--set', deviceSet, 'list', ...args, '--json'], { encoding: 'utf8', timeout: SharedSimulator.LIST_MILLISECONDS, maxBuffer: 32 * 1024 * 1024 })
    assert.equal(result.status, 0, `simctl list ${args.join(' ')}: ${result.error ?? ''}${result.stderr}`)
    return JSON.parse(result.stdout)
  }

  /** @param {Record<string, {udid: string, state: string, deviceTypeIdentifier: string}[]>} devices @param {string} udid @returns {[string, {udid: string, state: string, deviceTypeIdentifier: string}] | []} */
  static find(devices, udid) {
    for (const [runtime, list] of Object.entries(devices)) {
      const found = list.find((item) => item.udid === udid)
      if (found) return [runtime, found]
    }
    return []
  }

  /** @param {string} deviceSet @param {string} family @returns {SharedDevice} */
  static use(deviceSet, family) {
    const variable = SharedSimulator.VARIABLES[family]
    const device = SharedSimulator.udid(family)
    const [identifier, found] = SharedSimulator.find(SharedSimulator.list(deviceSet, ['devices']).devices, device)
    assert.ok(found, `${variable} ${device} is not in the device set ${deviceSet}`)
    assert.equal(found.state, 'Booted', `${variable} ${device} must already be booted`)
    const runtime = SharedSimulator.list(deviceSet, ['runtimes']).runtimes.find((item) => item.identifier === identifier)
    assert.ok(runtime && SharedSimulator.wantedRuntime(runtime), `${variable} ${device} runs ${identifier}, not iOS ${process.env.TAC_IOS_RUNTIME}`)
    const type = SharedSimulator.list(deviceSet, ['devicetypes']).devicetypes.find((item) => item.identifier === found.deviceTypeIdentifier)
    assert.equal(type?.productFamily, family, `${variable} ${device} must be an ${family}`)
    console.log(`==> using booted ${type.name} with iOS ${runtime.version} (${variable})`)
    return { device, runtime, type }
  }

  /** @param {string} deviceSet @param {string[]} args */
  static quiet(deviceSet, args) {
    spawnSync('/usr/bin/xcrun', ['simctl', '--set', deviceSet, ...args], { timeout: SharedSimulator.CLEANUP_MILLISECONDS, stdio: 'ignore' })
  }

  /** @param {string} deviceSet @param {string} device @param {string[]} applicationIds */
  static release(deviceSet, device, applicationIds) {
    for (const applicationId of applicationIds) {
      SharedSimulator.quiet(deviceSet, ['terminate', device, applicationId])
      SharedSimulator.quiet(deviceSet, ['privacy', device, 'reset', 'all', applicationId])
      SharedSimulator.quiet(deviceSet, ['uninstall', device, applicationId])
    }
  }

  /** @param {string} deviceSet @param {string} device */
  static remove(deviceSet, device) {
    SharedSimulator.quiet(deviceSet, ['shutdown', device])
    SharedSimulator.quiet(deviceSet, ['delete', device])
  }

  /** @param {string} deviceSet @param {string} device @param {string[]} applicationIds */
  static finish(deviceSet, device, applicationIds) {
    if (SharedSimulator.isShared(device)) return SharedSimulator.release(deviceSet, device, applicationIds)
    SharedSimulator.remove(deviceSet, device)
  }

  /** @param {string} deviceSet @param {string} device */
  static shutdown(deviceSet, device) {
    if (SharedSimulator.isShared(device)) return
    SharedSimulator.quiet(deviceSet, ['shutdown', device])
  }
}
