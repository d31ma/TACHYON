@Service
@Lifetime('transient')
class ProductsService {
  /** @returns {Product[]} */
  list() { return [this.find(1)]; }
  /**
   * @param {number} id
   * @returns {Product}
   */
  find(id) { return { id, name: 'Anvil' }; }
}
