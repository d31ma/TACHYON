@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
export default class ProductStore {
  /** @type {BehaviorContext} */
  @Needs
  behaviors;
  /**
   * @param {string} status
   * @returns {Promise<ProductEntity[] | void>}
   */
  async findByStatus(status) {}
  /**
   * @param {string} status
   * @returns {Promise<number | void>}
   */
  async countByStatus(status) {}
  /**
   * @param {string} status
   * @param {BehaviorPageInput} page
   * @returns {Promise<ProductEntity[] | void>}
   */
  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
  async excluding(status, @PageDefaults page) {}
  /**
   * @param {BehaviorPageInput} request
   * @returns {BehaviorPageInput}
   */
  bounded(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) request) { return request; }
}
