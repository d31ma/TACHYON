@Service
export default class CatalogService {
  /** @type {ProductsClient} */
  @Needs
  productsClient;
  /** @type {object[]} */
  products = [];
  async load() {
    const response = await fetch('/api/products', { credentials: 'omit' });
    this.products = await this.#productsFrom(response);
  }
  /**
   * @param {Event} _event
   * @param {BehaviorJsonValue} product
   */
  async save(_event, product) {
    await this.productsClient.save(product);
  }
  /**
   * @param {Response} response
   * @returns {Promise<object[]>}
   */
  #productsFrom(response) {
    if (!response.ok) throw new Error('Products unavailable');
    return response.json();
  }
}