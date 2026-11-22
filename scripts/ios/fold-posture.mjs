import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'

/**
 * @typedef {Object} LitScreen
 * @property {string} display
 */

export class FoldPosture {
  static OUTER_SCREEN = 'LCD'
  static INNER_SCREEN = 'LCD-1'
  static SETTLE_MILLISECONDS = 2000
  static COMMAND_MILLISECONDS = 60000
  static SCREENS = {
    closed: { on: FoldPosture.OUTER_SCREEN, off: FoldPosture.INNER_SCREEN },
    open: { on: FoldPosture.INNER_SCREEN, off: FoldPosture.OUTER_SCREEN },
  }

  /** @param {string} deviceSet @param {string} device @param {string} screen @param {string} power */
  static power(deviceSet, device, screen, power) {
    const args = ['simctl', '--set', deviceSet, 'io', device, 'screenConfig', `--display=${screen}`, 'power', power]
    const result = spawnSync('/usr/bin/xcrun', args, { encoding: 'utf8', timeout: FoldPosture.COMMAND_MILLISECONDS })
    assert.equal(result.status, 0, `simctl ${args.slice(3).join(' ')}: ${result.error || result.stderr || result.stdout}`)
  }

  /** @param {string} deviceSet @param {string} device @param {string} posture @returns {Promise<LitScreen>} */
  static async set(deviceSet, device, posture) {
    const screens = FoldPosture.SCREENS[posture]
    assert.ok(screens, `the ${posture} posture needs the hinge angle, which no public simulator command sets yet`)
    FoldPosture.power(deviceSet, device, screens.on, 'on')
    FoldPosture.power(deviceSet, device, screens.off, 'off')
    await new Promise((resolve) => setTimeout(resolve, FoldPosture.SETTLE_MILLISECONDS))
    return { display: screens.on }
  }
}
