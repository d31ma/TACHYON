@Store
class CatalogStore {
  @Needs
  DataSource dataSource;
  Object find() { return dataSource.query("SELECT 42 AS amount", java.util.List.of()); }
}