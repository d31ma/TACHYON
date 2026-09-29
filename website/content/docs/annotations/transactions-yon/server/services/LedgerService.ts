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
    return { open: LedgerService.rows(open), total: LedgerService.count(total) };
  }

  private static rows(open: BehaviorJsonObject[] | void): BehaviorJsonObject[] {
    return open ?? [];
  }

  private static count(total: number | void): number {
    return total ?? 0;
  }
}
