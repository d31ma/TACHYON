@Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100})
class ProductStore {
  /** @type {BehaviorContext} */
  @Needs
  behaviors;
  /** @param {string} status */
  async findByStatus(status) {}
  /** @param {string} status */
  async countByStatus(status) {}
  /**
   * @param {string} status
   * @param {BehaviorPageInput} page
   */
  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})
  async excluding(status, @PageDefaults page) {}
  /**
   * @param {BehaviorPage} request
   * @returns {BehaviorPage}
   */
  bounded(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) request) { return request; }
}
