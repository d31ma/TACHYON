/** @import { CountChanged } from '/models/CountChanged.ts' */
@Service
export default class CounterService {
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  count = 0;

  add() {
    this.count += 1;
    /** @type {CountChanged} */
    const changed = { count: this.count };
    this.applicationEventPublisher.publishEvent(changed);
  }
}
