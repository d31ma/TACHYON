@Store
class CatalogStore {
  /** @type {DataSource} */
  @Needs
  dataSource;
  /** @returns {Promise<object[]>} */
  find() { return this.dataSource.query('SELECT 42 AS amount', []); }
}