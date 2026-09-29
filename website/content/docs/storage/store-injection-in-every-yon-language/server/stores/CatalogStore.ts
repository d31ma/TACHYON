@Store
export default class CatalogStore {
  @Needs
  dataSource!: DataSource;
  find() { return this.dataSource.query('SELECT 42 AS amount', []); }
}