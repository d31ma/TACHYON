[Store]
class CatalogStore {
  [Needs]
  DataSource _dataSource;
  public object find() { return _dataSource.Query("SELECT 42 AS amount", new object[] {}); }
}