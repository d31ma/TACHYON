@Service
export default class CatalogService {
  /** @type {CatalogClient} */
  @Needs
  catalogClient;
  /** @type {unknown[]} */
  products = []
  error = ''

  async load() {
    await this.catalogClient.all().then((products) => this._show(products), () => this._showFailure())
  }

  /**
   * @param {unknown} products
   * @returns {void}
   */
  _show(products) {
    if (!Array.isArray(products)) return this._showFailure()
    this.products = products
    this.error = ''
  }

  _showFailure() {
    this.error = 'Products could not be loaded. Please retry.'
  }
}