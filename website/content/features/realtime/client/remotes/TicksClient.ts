import type { Tick } from '/models/Tick.ts';

@HTTPClient('/ticks')
export default class TicksClient {
  @StreamRequest
  streamTicks(tick: Tick): void {}
}
