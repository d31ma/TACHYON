@Service
export default class TaxService {
  /**
   * @param {string} region
   * @returns {number}
   */
  rate(region) {
    return 2;
  }

  /**
   * @param {number} count
   * @returns {number}
   */
  total(count) {
    return count * 10;
  }
}
