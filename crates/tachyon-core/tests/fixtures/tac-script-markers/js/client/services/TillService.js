/** @import { LedgerEntry } from '/models/LedgerEntry.js' */
@Service
export default class TillService {
  /** @type {ApplicationEventPublisher} */
  applicationEventPublisher;
  /** @type {string[]} */
  steps = [];

  @OnStart
  start() { this.steps = [...this.steps, 'start']; }

  @OnStop
  stop() { this.steps = [...this.steps, 'stop']; }

  /** @param {number} amount */
  post(amount) {
    /** @type {LedgerEntry} */
    const entry = { amount };
    this.applicationEventPublisher.publishEvent(entry);
  }
}
