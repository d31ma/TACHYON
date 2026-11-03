import assert from 'node:assert/strict'

export class SnippetShard {
  static pattern = /^([1-9][0-9]*)\/([1-9][0-9]*)$/
  static usage = 'Use --shard=<index>/<count>, for example --shard=2/3'
  static whole = '1/1'

  /** @type {number} */
  index
  /** @type {number} */
  count
  /** @type {number} */
  offered = 0

  /**
   * @param {number} index
   * @param {number} count
   */
  constructor(index, count) {
    this.index = index
    this.count = count
  }

  /**
   * @param {string|undefined} text
   * @returns {SnippetShard}
   */
  static parse(text) {
    return SnippetShard.#fromText(SnippetShard.#writtenOrWhole(text))
  }

  /**
   * @template T
   * @param {T[]} steps
   * @returns {T[]}
   */
  select(steps) {
    return steps.filter(() => this.#takesNext())
  }

  /** @returns {boolean} */
  ownsSingleChecks() {
    return this.index === 1
  }

  /** @returns {string} */
  describe() {
    return `${this.index} of ${this.count}`
  }

  /**
   * @param {string|undefined} text
   * @returns {string}
   */
  static #writtenOrWhole(text) {
    return text ?? SnippetShard.whole
  }

  /**
   * @param {string} text
   * @returns {SnippetShard}
   */
  static #fromText(text) {
    const match = SnippetShard.pattern.exec(text)
    assert.ok(match, `${SnippetShard.usage}, not ${text}`)
    const shard = new SnippetShard(Number(match[1]), Number(match[2]))
    assert.ok(shard.index <= shard.count, `${SnippetShard.usage}: the index ${shard.index} is above the count ${shard.count}`)
    return shard
  }

  /** @returns {boolean} */
  #takesNext() {
    const position = this.offered
    this.offered += 1
    return position % this.count === this.index - 1
  }
}
