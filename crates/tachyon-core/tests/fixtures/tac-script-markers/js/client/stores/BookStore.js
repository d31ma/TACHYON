/** @extends {Repository<BookEntity, string>} */
@Store(BookEntity, {"fields":["code","status"],"defaultSize":2,"maxSize":3,"setup":"ShelfSetup"})
export default class BookStore extends Repository {
  /**
   * @param {string} status
   * @returns {Promise<BookEntity[] | void>}
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
   * @returns {Promise<BookEntity[] | void>}
   */
  @Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"code","direction":"desc"}]})
  async excluding(status, @PageDefaults page) {}
}
