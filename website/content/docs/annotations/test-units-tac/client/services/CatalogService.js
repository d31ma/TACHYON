@Service
export default class CatalogService {
  title = 'Catalog';
  /**
   * @param {number} value
   * @returns {number}
   */
  double(value) { return value * 2; }
}
