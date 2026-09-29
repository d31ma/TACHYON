@Service
export default class ProductService {
  /** @type {ProductStore} */
  @Needs
  productStore;
  summary = 'Not loaded.';

  async load() {
    const rows = await this.productStore.excluding('retired', { size: 5 });
    const activeCount = await this.productStore.countByStatus('active');
    this.summary = this.#summaryOf(rows, activeCount);
  }

  /**
   * @param {ProductEntity[] | void} rows
   * @param {number | void} activeCount
   * @returns {string}
   */
  #summaryOf(rows, activeCount) {
    return `${rows ? rows.length : 0} of ${activeCount} active`;
  }
}
