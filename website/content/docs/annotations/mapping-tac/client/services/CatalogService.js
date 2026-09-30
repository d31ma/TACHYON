@Service
export default class CatalogService {
  /** @type {ProductConverter} */
  @Needs
  productConverter;
  title = 'Not mapped.';

  async load() {
    const product = { detail: { id: 7 }, name: 'Anvil', extra: 'dropped' };
    const summary = await this.productConverter.toSummary(product);
    this.title = this.#titleOf(summary);
  }

  /**
   * @param {BehaviorJsonObject | void} summary
   * @returns {string}
   */
  #titleOf(summary) {
    return summary ? `${summary.kind} ${summary.productId}: ${summary.title}` : '';
  }
}
