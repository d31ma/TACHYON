import type { Tick } from '/models/Tick.ts';

@Service
export default class StreamService {
  @Needs
  readonly ticksClient!: TicksClient;
  ticks: Tick[] = []

  @OnEvent
  addTick(tick: Tick): void {
    this.ticks = [...this.ticks.slice(-127), tick]
  }
}
