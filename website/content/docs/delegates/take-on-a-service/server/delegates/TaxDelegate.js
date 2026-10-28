@Delegate
export default class TaxDelegate {
  /** @type {TaxService} */
  @Needs
  taxService;

  /**
   * @param {string} region
   * @returns {number}
   */
  rate(region) {
    return this.taxService.rate(region) * 2;
  }

  /**
   * @param {number} count
   * @param {number} extra
   * @returns {number}
   */
  total(count, extra) {
    return this.taxService.total(count) + extra;
  }
}
