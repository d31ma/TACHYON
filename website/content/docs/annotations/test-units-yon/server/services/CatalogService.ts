@Service
class CatalogService {
  @Needs
  catalogCounterHelper!: CatalogCounterHelper;

  total(): number { return this.catalogCounterHelper.count() * 2; }
}
