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
    return { open, total };
  }
}
