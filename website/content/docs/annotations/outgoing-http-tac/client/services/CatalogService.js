@Service
export default class CatalogService {
  /** @type {CatalogClient} */
  @Needs
  catalogClient;
  count = 0;

  async load() {
    const answer = await this.catalogClient.all();
    this.count = this.#countItems(answer.items);
  }

  /**
   * @param {unknown} items
   * @returns {number}
   */
  #countItems(items) {
    return Array.isArray(items) ? items.length : 0;
  }
}
