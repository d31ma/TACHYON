@Service
export default class CatalogService {
  /** @type {ProductConverter} */
  @Needs
  productConverter;
  /** @type {string} */
  result = '';

  /** @returns {Promise<void>} */
  async load() {
    const detail = {name: 'detail', secret: 'detail-private'};
    const product = await this.productConverter.copy({id: 7, name: 'Anvil', detail, items: [detail], secret: 'product-private'});
    const summary = await this._summaryOf(product);
    this.result = this._text(summary);
  }

  /**
   * @param {Product | void} product
   * @returns {Promise<Summary | void>}
   */
  async _summaryOf(product) {
    return product ? this.productConverter.map(product) : undefined;
  }

  /**
   * @param {Summary | void} summary
   * @returns {string}
   */
  _text(summary) {
    return summary ? `${summary.kind} ${summary.id}: ${summary.name}, ${summary.detail.name}, ${summary.items.length}` : '';
  }
}
