@Service
export default class LedgerService {
  behaviors!: BehaviorContext;
  @Needs
  readonly entryStore!: EntryStore;

  @OnStart
  async open() {
    const signal = new AbortController().signal;
    this.behaviors = await Behaviors.create(LedgerSetup, { partition: 'ledger', signal });
  }

  @AllOrNothing({"adapter":"ledger"})
  async settle(status: string) {
    const open = await this.entryStore.findByStatus(status);
    const total = await this.entryStore.countByStatus(status);
    return { open, total };
  }
}
