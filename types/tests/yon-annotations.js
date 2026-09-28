@Store
class CatalogRepository {
  items = ['one', 'two'];
}

@Service()
@Needs(CatalogRepository)
class CatalogService {
  /** @param {CatalogRepository} repository */
  constructor(repository) { this.repository = repository; }
}

@RequestRoute('/catalog')
@Needs(CatalogService)
class Catalog {
  /** @param {CatalogService} service */
  constructor(service) { this.service = service; }

  @GetRoute
  getCatalog() { return this.service.repository.items; }
}

export {};
