@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class EntryStore {
  @Needs
  behaviors!: BehaviorContext;
  async findByStatus(status: string) {}
  async countByStatus(status: string) {}

  @AllOrNothing({"adapter":"ledger"})
  async settle(status: string) {
    const open = await this.findByStatus(status);
    const total = await this.countByStatus(status);
    return { open, total };
  }
}
