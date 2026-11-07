/** @import { Tick } from '/models/Tick.ts' */
@RequestRoute("/ticks")
@Public
class TicksController {
  /** @returns {AsyncGenerator<Tick>} */
  @StreamRoute
  async *streamTicks() {
    yield* this._countdown(3)
  }

  /**
   * @param {number} start
   * @returns {AsyncGenerator<Tick>}
   */
  async *_countdown(start) {
    for (let level = start; level > 0; level -= 1) {
      yield { sku: 'ANVIL-1', level }
      await new Promise((resume) => setTimeout(resume, 1000))
    }
  }
}
