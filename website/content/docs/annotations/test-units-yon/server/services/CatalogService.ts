@Service
export default class CatalogService {
  @Needs
  catalogCounterDelegate!: CatalogCounterDelegate;

  total(): number { return this.catalogCounterDelegate.count() * 2; }
}
