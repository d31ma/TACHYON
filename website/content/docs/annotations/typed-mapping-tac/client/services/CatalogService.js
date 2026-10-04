@Service
export default class CatalogService {
  /** @type {ProductConverter} */
  @Needs
  productConverter;
  /** @type {string} */
  result = '';

  /** @returns {Promise<void>} */
  async load() {
    const detail = {name: 'detail', secret: 'hidden'};
    const product = await this.productConverter.copy({id: 7, name: 'Anvil', detail, items: [detail], secret: 'hidden'});
    this.result = await this._summaryText(product);
  }

  /**
   * @param {Product | void} product
   * @returns {Promise<string>}
   */
  async _summaryText(product) {
    return product ? JSON.stringify(await this.productConverter.map(product)) : '';
  }
}
