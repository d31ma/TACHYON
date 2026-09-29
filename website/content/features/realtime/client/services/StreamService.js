/** @import { Tick } from '/models/Tick.ts' */
@Service
export default class StreamService {
  /** @type {TicksClient} */
  @Needs
  ticksClient;
  /** @type {Tick[]} */
  ticks = []

  /** @param {Tick} tick */
  @OnEvent
  addTick(tick) {
    this.ticks = [...this.ticks.slice(-127), tick]
  }
}
