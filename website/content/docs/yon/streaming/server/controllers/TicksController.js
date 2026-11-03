/** @import { Tick } from '/models/Tick.ts' */
@RequestRoute("/ticks")
@Public
class TicksController {
  /** @returns {AsyncGenerator<Tick>} */
  @StreamRoute
  async *streamTicks() {
    yield* this._ticksUpTo(3)
  }

  /**
   * @param {number} last
   * @returns {AsyncGenerator<Tick>}
   */
  async *_ticksUpTo(last) {
    for (let level = 1; level <= last; level += 1) {
      yield { sku: 'ANVIL-1', level }
    }
  }
}
