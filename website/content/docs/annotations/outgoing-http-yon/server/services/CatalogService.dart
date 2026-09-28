@Service
class CatalogService {
  @Needs
  final CatalogClient catalog;

  Future<Object?> products() {
    return catalog.all(1);
  }
}
