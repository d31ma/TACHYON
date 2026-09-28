/** @import { Tick } from '/models/Tick.ts' */
@HTTPClient('/ticks')
class TicksClient {
  /** @param {Tick} tick */
  @StreamRequest
  streamTicks(tick) {}
}
