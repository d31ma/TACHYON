import type { Tick } from '/models/Tick.ts'

@RequestRoute("/ticks")
@Public
class TicksController {
  @StreamRoute
  async *streamTicks(): AsyncGenerator<Tick> {
    yield* this.countdown(3)
  }

  private async *countdown(start: number): AsyncGenerator<Tick> {
    for (let level = start; level > 0; level -= 1) {
      yield { sku: 'ANVIL-1', level }
      await new Promise((resume) => setTimeout(resume, 1000))
    }
  }
}
