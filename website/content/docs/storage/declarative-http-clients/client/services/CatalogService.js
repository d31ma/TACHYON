@Service
export default class CatalogService {
  /** @type {CatalogClient} */
  @Needs
  catalogClient;
  /** @type {unknown[]} */
  products = []
  error = ''

  async load() {
    await this.catalogClient.all().then((products) => this.#show(products), () => this.#showFailure())
  }

  /**
   * @param {unknown} products
   * @returns {void}
   */
  #show(products) {
    if (!Array.isArray(products)) return this.#showFailure()
    this.products = products
    this.error = ''
  }

  #showFailure() {
    this.error = 'Products could not be loaded. Please retry.'
  }
}