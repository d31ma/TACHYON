@Store(EntryEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"LedgerSetup"})
export default class EntryStore {
  /**
   * @param {string} status
   * @returns {Promise<EntryEntity[] | void>}
   */
  async findByStatus(status) {}
  /**
   * @param {string} status
   * @returns {Promise<number | void>}
   */
  async countByStatus(status) {}
}
