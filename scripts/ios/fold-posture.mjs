import assert from 'node:assert/strict'

/**
 * @typedef {Object} LitScreen
 * @property {string} display
 */

export class FoldPosture {

  /** @param {string} deviceSet @param {string} device @param {string} posture @returns {Promise<LitScreen>} */
  static async set(deviceSet, device, posture) {
    assert.fail(`the ${posture} posture of ${device} in ${deviceSet} needs the simulator hinge, and no public simctl command moves it; simctl io screenConfig power only turns a display off and crashes the system shell`)
  }
}
