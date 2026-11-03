@Service
export default class CatalogService {
  @Needs
  dataSource!: DataSource;
  find() { return this.dataSource.query('SELECT 42 AS amount', []); }
}