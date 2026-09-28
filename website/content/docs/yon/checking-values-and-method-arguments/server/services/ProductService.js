@Service
class ProductService {
  /**
   * @param {ProductInput} input
   * @returns {ProductInput}
   */
  accept(@Check input) {
    return input;
  }
}
