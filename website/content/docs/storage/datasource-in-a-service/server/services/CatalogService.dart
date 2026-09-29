@Service
class CatalogService {
  @Needs
  final DataSource dataSource;

  Future<Object?> find() {
    return dataSource.query('SELECT 42 AS amount', []);
  }
}
