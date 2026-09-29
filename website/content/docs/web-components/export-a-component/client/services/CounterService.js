/** @import { CountChanged } from '/models/CountChanged.ts' */
@Service
@NeedsAll
export default class CounterService {
  /** @type {string} */
  start;
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  count = 0;

  @OnStart
  initialize() {
    this.count = Number(this.start);
  }

  add() {
    this.count += 1;
    /** @type {CountChanged} */
    const changed = { count: this.count };
    this.applicationEventPublisher.publishEvent(changed);
  }
}
