@Service
export default class CatalogService {
  /** @type {DataSource} */
  @Needs
  dataSource;
  /** @returns {Promise<object[]>} */
  find() { return this.dataSource.query('SELECT 42 AS amount', []); }
}