class TacTaskAnswers {
  /** @type {Map<number, (text: string) => void>} */
  #waiting = new Map()
  /** @type {Map<number, string>} */
  #early = new Map()

  /** @param {number} task @returns {Promise<string>} */
  wait(task) {
    const early = this.#early.get(task)
    this.#early.delete(task)
    if (early !== undefined) return Promise.resolve(early)
    return new Promise((resolve) => { this.#waiting.set(task, resolve) })
  }

  /** @param {string} text @returns {Promise<string>} */
  answer(text) {
    const task = JSON.parse(text)?.task
    return Number.isSafeInteger(task) ? this.wait(task) : Promise.resolve(text)
  }

  /** @param {number} task @param {string} text @returns {void} */
  done(task, text) {
    const resolve = this.#waiting.get(task)
    this.#waiting.delete(task)
    if (resolve) resolve(text)
    else this.#early.set(task, text)
  }
}
