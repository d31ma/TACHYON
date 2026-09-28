@Service
class CatalogService {
  /** @type {CatalogCounterHelper} */
  @Needs
  catalogCounterHelper;

  /** @returns {number} */
  total() { return this.catalogCounterHelper.count() * 2; }
}
