@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class EntryStore {
  /** @type {BehaviorContext} */
  @Needs
  behaviors;
  /** @param {string} status */
  async findByStatus(status) {}
  /** @param {string} status */
  async countByStatus(status) {}

  /**
   * @param {string} status
   * @returns {Promise<{ open: unknown, total: unknown }>}
   */
  @AllOrNothing({"adapter":"ledger"})
  async settle(status) {
    const open = await this.findByStatus(status);
    const total = await this.countByStatus(status);
    return { open, total };
  }
}
