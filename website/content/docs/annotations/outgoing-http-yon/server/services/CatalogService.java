@Service
class CatalogService {
    @Needs
    final CatalogClient catalog;
    Object products() { return catalog.all(1L); }
}
