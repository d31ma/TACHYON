@Service
export default class CatalogService {
  /** @type {ProductsClient} */
  @Needs
  productsClient;
  count = 0;

  async load() {
    this.count = this._countItems(await this.productsClient.list());
  }

  async add() {
    await this.productsClient.save({ name: 'anvil' });
  }

  /**
   * @param {{ items: string[] } | void} answer
   * @returns {number}
   */
  _countItems(answer) {
    return answer ? answer.items.length : 0;
  }
}
