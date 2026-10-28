@Service
export default class CatalogService {
  /** @type {CatalogCounterDelegate} */
  @Needs
  catalogCounterDelegate;

  /** @returns {number} */
  total() { return this.catalogCounterDelegate.count() * 2; }
}
