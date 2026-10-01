@Entity({ adapter: 'memory' })
class CatalogItem {
  @Id
  id = '';
}

/** @extends {Repository<CatalogItem, string>} */
@Store(CatalogItem, { fields: ['id'], defaultSize: 20, maxSize: 100 })
class CatalogRepository extends Repository {}

// @ts-expect-error A Store must name its Entity and query options.
@Store
class UnconfiguredStore {}

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
  getCatalog() { return this.service.repository.findAll({}); }
}

export {};
