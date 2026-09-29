@Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})
export default class BillingStore {
  /**
   * @param {number} id
   * @returns {Promise<boolean | void>}
   */
  async existsById(id) {}
}
