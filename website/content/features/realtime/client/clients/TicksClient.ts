import type { Tick } from '/models/Tick';

@HTTPClient('/ticks')
class TicksClient {
  @StreamRequest
  streamTicks(tick: Tick): void {}
}
