@Service
export default class ShelfService {
  static _ACTIVE = 'active';
  static _FULL = 'the shelf is full';
  /** @type {BookStore} */
  @Needs
  bookStore;
  /** @type {AbortSignal} */
  @Needs
  lifecycle;
  /** @type {BehaviorContext} */
  behaviors;

  @OnStart
  async open() {
    this.behaviors = await Behaviors.create(ShelfSetup, { partition: 'shelf', signal: this.lifecycle });
  }

  /** @returns {Promise<BehaviorJsonValue>} */
  async report() {
    const active = await this.bookStore.findByStatus(ShelfService._ACTIVE);
    const count = await this.bookStore.countByStatus(ShelfService._ACTIVE);
    const others = await this.bookStore.excluding(ShelfService._ACTIVE, {});
    const full = await this.bookStore.excluding(ShelfService._ACTIVE, { size: 3 });
    return { active, count, others, full };
  }

  /** @returns {Promise<BookEntity[] | void>} */
  async oversized() {
    return this.bookStore.excluding(ShelfService._ACTIVE, { size: 4 });
  }

  /**
   * @param {BehaviorPageInput} input
   * @returns {BehaviorPageInput}
   */
  page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["code"]}) input) {
    return input;
  }

  /**
   * @param {string} code
   * @param {string} status
   * @returns {Promise<BehaviorJsonValue>}
   */
  @AllOrNothing({"adapter":"shelf"})
  async shelve(code, status) {
    return this.behaviors.repository(BookEntity).save({ code, status });
  }

  /**
   * @param {string} code
   * @returns {Promise<BehaviorJsonValue>}
   */
  @AllOrNothing({"adapter":"shelf"})
  async refuse(code) {
    await this.behaviors.repository(BookEntity).save({ code, status: 'draft' });
    throw Error(ShelfService._FULL);
  }
}
