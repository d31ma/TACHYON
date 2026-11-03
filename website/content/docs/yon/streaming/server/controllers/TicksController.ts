import type { Tick } from '/models/Tick.ts'

@RequestRoute("/ticks")
@Public
class TicksController {
  @StreamRoute
  async *streamTicks(): AsyncGenerator<Tick> {
    yield* this.ticksUpTo(3)
  }

  private async *ticksUpTo(last: number): AsyncGenerator<Tick> {
    for (let level = 1; level <= last; level += 1) {
      yield { sku: 'ANVIL-1', level }
    }
  }
}
