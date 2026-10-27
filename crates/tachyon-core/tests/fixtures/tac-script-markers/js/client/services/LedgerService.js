/** @import { LedgerEntry } from '/models/LedgerEntry.js' */
@Service
export default class LedgerService {
  /** @type {string[]} */
  heard = [];

  /** @param {LedgerEntry} entry */
  @OnEvent
  @Order(5)
  late(entry) { this.heard = [...this.heard, `late ${entry.amount}`]; }

  /** @param {LedgerEntry} entry */
  @OnEvent
  @Order(-5)
  early(entry) { this.heard = [...this.heard, `early ${entry.amount}`]; }

  /** @param {LedgerEntry} entry */
  @OnEvent
  @InBackground
  later(entry) { this.heard = [...this.heard, `background ${entry.amount}`]; }
}
