@Entity({ adapter: 'memory' })
class InventoryItem {
  @Id
  id = '';
}

@Store(InventoryItem, { fields: ['id'], defaultSize: 20, maxSize: 100 })
class Inventory extends Repository<InventoryItem, string> {}

// @ts-expect-error A Store must name its Entity and query options.
@Store()
class UnconfiguredStore {}

@HTTPClient
class RemoteCatalog {}

@Delegate()
class ExternalCatalog {}

@Service()
@Lifetime('singleton')
@Needs(Inventory, RemoteCatalog, ExternalCatalog)
class ProductsService {
  constructor(
    readonly inventory: Inventory,
    readonly remote: RemoteCatalog,
    readonly external: ExternalCatalog,
  ) {}
}

@Service
@Lifetime('transient')
class TraceContext {}

@RequestRoute('/products')
@Lifetime('request')
@Needs(ProductsService, TraceContext)
class Products {
  constructor(readonly products: ProductsService, readonly trace: TraceContext) {}

  @GetRoute
  async getProducts(request: { method: string }) {
    return { method: request.method, products: await this.products.inventory.findAll({}) };
  }

  @PostRoute
  postProduct() { return { created: true }; }

  @PutRoute
  putProduct() { return { replaced: true }; }

  @PatchRoute
  patchProduct() { return { changed: true }; }

  @DeleteRoute
  deleteProduct() { return { removed: true }; }
}

@RequestRoute('/updates')
class Updates {
  @StreamRoute()
  async *streamUpdates() { yield { update: true }; }
}

@RequestRoute('/audit')
class Audit {
  @StreamRoute('/all')
  *streamAudit() { yield { audit: true }; }
}

@RequestRoute('/plain')
class PlainStream {
  // @ts-expect-error StreamRoute needs a method that yields its messages.
  @StreamRoute
  streamPlain() { return { plain: true }; }
}

@RequestRoute('/retired')
class RetiredStream {
  @GetRoute
  // @ts-expect-error Stream was replaced by StreamRoute.
  @Stream
  *getRetired() { yield { retired: true }; }
}

// @ts-expect-error Lifetime accepts only the three implemented lifecycle names.
@Lifetime('session')
class UnsupportedScope {}

// @ts-expect-error Injection tokens are constructor values, not names or expressions.
@Needs('ProductsService')
class StringInjection {}

abstract class AbstractService {}

// @ts-expect-error Direct constructor injection requires a concrete provider class.
@Needs(AbstractService)
class AbstractInjection {}

// @ts-expect-error Controller is retired; RequestRoute declares controller paths.
@Controller('/products')
class DecoratorRoute {}

class WrongTarget {
  // @ts-expect-error RequestRoute markers belong on classes, not methods.
  @RequestRoute()
  method() {}

  // @ts-expect-error StreamRoute markers belong on methods, not fields.
  @StreamRoute()
  field = 'not a method';
}

class RemovedHttpMarkers {
  // @ts-expect-error GET annotations were removed; name the method GET instead.
  @GET()
  GET() {}
  // @ts-expect-error POST annotations were removed; name the method POST instead.
  @POST
  POST() {}
  // @ts-expect-error PUT annotations were removed; name the method PUT instead.
  @PUT()
  PUT() {}
  // @ts-expect-error PATCH annotations were removed; name the method PATCH instead.
  @PATCH
  PATCH() {}
  // @ts-expect-error DELETE annotations were removed; name the method DELETE instead.
  @DELETE()
  DELETE() {}
  // @ts-expect-error OPTIONS annotations were removed; name the method OPTIONS instead.
  @OPTIONS
  OPTIONS() {}
  // @ts-expect-error HEAD annotations were removed; name the method HEAD instead.
  @HEAD()
  HEAD() {}
}

export {};
