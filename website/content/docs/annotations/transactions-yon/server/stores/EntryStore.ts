@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
export default class EntryStore {
  @Needs
  behaviors!: BehaviorContext;
  async findByStatus(status: string): Promise<BehaviorJsonObject[] | void> {}
  async countByStatus(status: string): Promise<number | void> {}

  @AllOrNothing({"adapter":"ledger"})
  async settle(status: string) {
    const open = (await this.findByStatus(status)) ?? [];
    const total = (await this.countByStatus(status)) ?? 0;
    return { open, total };
  }
}
