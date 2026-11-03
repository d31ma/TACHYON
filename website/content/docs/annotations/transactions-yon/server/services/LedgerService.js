@Service
export default class LedgerService {
  /** @type {BehaviorContext} */
  behaviors;
  /** @type {EntryStore} */
  @Needs
  entryStore;

  @OnStart
  async open() {
    const signal = new AbortController().signal;
    this.behaviors = await Behaviors.create(LedgerSetup, { partition: 'ledger', signal });
  }

  /** @param {string} status */
  @AllOrNothing({"adapter":"ledger"})
  async settle(status) {
    const open = await this.entryStore.findByStatus(status);
    const total = await this.entryStore.countByStatus(status);
    return { open: LedgerService._rows(open), total: LedgerService._count(total) };
  }

  /**
   * @param {BehaviorJsonObject[] | void} open
   * @returns {BehaviorJsonObject[]}
   */
  static _rows(open) {
    return open ?? [];
  }

  /**
   * @param {number | void} total
   * @returns {number}
   */
  static _count(total) {
    return total ?? 0;
  }
}
