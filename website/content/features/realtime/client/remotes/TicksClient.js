/** @import { Tick } from '/models/Tick.ts' */
@HTTPClient('/ticks')
export default class TicksClient {
  /** @param {Tick} tick */
  @StreamRequest
  streamTicks(tick) {}
}
