@Service
export default class HintService {
  static _removedEvery = 3
  static _removedAt = 2

  /** @type {number} */
  turn = 0

  /** @type {string | null} */
  text = 'Hint 0'

  /** @returns {void} */
  bump() {
    this.turn += 1
    this.text = this.#textFor(this.turn)
  }

  /** @param {number} turn @returns {string | null} */
  #textFor(turn) {
    if (turn % HintService._removedEvery === HintService._removedAt) return null
    return `Hint ${turn}`
  }
}
