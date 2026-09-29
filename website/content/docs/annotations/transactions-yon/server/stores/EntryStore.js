@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
export default class EntryStore {
  /** @type {BehaviorContext} */
  @Needs
  behaviors;
  /**
   * @param {string} status
   * @returns {Promise<BehaviorJsonObject[] | void>}
   */
  async findByStatus(status) {}
  /**
   * @param {string} status
   * @returns {Promise<number | void>}
   */
  async countByStatus(status) {}

  /** @param {string} status */
  @AllOrNothing({"adapter":"ledger"})
  async settle(status) {
    const open = (await this.findByStatus(status)) ?? [];
    const total = (await this.countByStatus(status)) ?? 0;
    return { open, total };
  }
}
