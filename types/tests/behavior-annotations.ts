// @ts-expect-error Tachyon annotations and types are global, so the old module no longer exists (ADR 0046).
import type { BehaviorJsonValue as RemovedModuleValue } from 'tachyon:behaviors';
type RemovedModule = RemovedModuleValue;

@Service
@NeedsAll
class GlobalMarkerService {
  readonly settings!: string;
  @Id
  id = 0;
}

@Service()
@NeedsAll()
class NamedFieldService {
  readonly settings!: string;
}

@RequestRoute
@Schedule({"fixedDelaySeconds":30})
class ScheduledWorker {}
void GlobalMarkerService; void NamedFieldService; void ScheduledWorker;

// @ts-expect-error The former instance-wide TacRuntime contract has been removed.
type RemovedTacRuntime = TacRuntime;

@Settings({"prefix":"YON_CATALOG_"})
class CatalogSettings {
  @Min(1)
  @Max(100)
  size: integer = 20;
  title!: string;
}

@HTTPClient({"cache":"network-first","adapter":"api"})
class ProductsClient {
  @GetRequest("/products")
  async list(): Promise<BehaviorJsonValue | void> {}
}
class ProductsPage {
  @Needs()
  readonly productsClient!: ProductsClient;
  async load(): Promise<BehaviorJsonValue | void> { return this.productsClient.list(); }
}
void ProductsPage;

class ApplicationValues {
  @Setting("${yon.server.port:8080}")
  port!: number;
  @Setting({"path":"yon.label","type":"string","default":"Products"})
  label?: string;
}
class ApplicationValueChecks {
  static applicationValues: Record<string, BehaviorJsonValue> = Behaviors.values(ApplicationValues, {});
  static applicationSettings = Object.assign(new ApplicationValues(), ApplicationValueChecks.applicationValues);
  static applicationPort: number = ApplicationValueChecks.applicationSettings.port;
}
void ApplicationValueChecks;

@Checked()
class ProductInput {
  @Required()
  @NotEmpty()
  @Length({"min":1,"max":100})
  name = '';
  @Min(0)
  @Max(100)
  price = 0;
}
class FactoryChecks {
  static boundSettings: Record<string, BehaviorJsonValue> = Behaviors.properties(CatalogSettings, { yon: { catalog: { size: 20 } } });
  static validatedInput: Record<string, BehaviorJsonValue> = Behaviors.validate(ProductInput, { name: 'Product', price: 10 });
}
void FactoryChecks;

class Adapters {
  static cacheAdapter: CacheAdapter = {
    get(_key, execution) {
      const partition: string = execution.partition;
      void partition;
      return { hit: false };
    },
    async set(_key, _value, execution) {
      const policy: [string, number, number, number, number] = [
        execution.name, execution.ttlMs, execution.maxBytes, execution.maxEntries, execution.expiresAt,
      ];
      void policy;
    },
    delete(_key, execution) { execution.signal.throwIfAborted(); },
  };

  static httpAdapter: HttpAdapter = {
    async exchange(request, execution) {
      const policy: ['error', 'omit', 1048576] = [request.redirect, request.credentials, request.maxResponseBytes];
      const operationDeadline: number = execution.deadline;
      void policy;
      void operationDeadline;
      return { products: [{ id: 1 }] };
    },
  };

  static entityAdapter: EntityAdapter = {
    execute(command, execution) {
      execution.signal.throwIfAborted();
      if (command.operation === 'transaction') return { transaction: 't1' };
      if (command.operation === 'find') return { offset: command.page.offset, rows: [] };
      if (command.operation === 'save') return command.value;
      if (command.operation === 'query') return [];
      return { deleted: command.id };
    },
  };
}

@Setup()
@WhenProfile('test')
@WhenSetting({"name":"YON_ENABLED","havingValue":"true","matchIfMissing":false})
class AppConfiguration {
  static configure(execution: BehaviorExecution): BehaviorConfiguration {
    return {
      caches: { products: Adapters.cacheAdapter },
      http: { catalog: Adapters.httpAdapter },
      entities: { data: Adapters.entityAdapter },
      authorizeEntity(command, _execution) { return command.operation === 'query'; },
      async authorize(call, operation) {
        const keyArguments: readonly BehaviorJsonValue[] = call.keyArguments;
        void keyArguments;
        return operation.partition === execution.partition && call.cache === 'products';
      },
    };
  }
}

@Setup()
class HttpConfiguration {
  static async configure(_execution: BehaviorExecution): Promise<BehaviorConfiguration> {
    return { http: { catalog: Adapters.httpAdapter } };
  }
}

@Entity({"adapter":"data"})
@Table("products")
class Product {
  @Id
  id = 0;
  label = '';
}

@Store(Product, {"fields":["id"],"defaultSize":20,"maxSize":100})
class ProductQueries {
  @Query({"operation":"exists","where":[{"field":"id","operator":"eq","argument":0}],"orderBy":[]})
  async available(_id: number) {}
}
void ProductQueries;

@Store(Product, {"fields":["id"],"defaultSize":20,"maxSize":100})
class ProductRepository extends Repository<Product, number> {
  async deleteByLabel(_label: string): Promise<number | void> {}
}
async function useProductRepository(products: ProductRepository): Promise<void> {
  const found: Product | null = await products.findById(7);
  const page: Product[] = await products.findAll({ size: 10 });
  const saved: Product = await products.save(new Product());
  const deleted: number = await products.deleteById(7);
  // @ts-expect-error A Repository id has the Id type of the Store.
  await products.findById('seven');
  void [found, page, saved, deleted];
}
void useProductRepository;

class GeneratedArguments {
  @Needs()
  @Named("CatalogSettings")
  readonly configuration!: CatalogSettings;
}
void GeneratedArguments;

@HTTPClient({"adapter":"catalog"})
class ProductRemoteClient {
  @GetRequest("/products")
  async remote(): Promise<BehaviorJsonValue | void> {}
}
void ProductRemoteClient;

@CacheSettings({"name":"products","ttlMs":1000,"maxBytes":4096,"maxEntries":20})
class ProductService {
  @Needs()
  behaviors!: BehaviorContext;
  @Needs()
  label!: string;

  @Cached
  async load(id: number): Promise<BehaviorJsonValue> {
    return { id, label: this.label };
  }
  @UpdateCache({"method":"load"})
  async replace(id: number): Promise<BehaviorJsonValue> { return { id }; }
  @ClearCache({"method":"load"})
  async remove(id: number): Promise<BehaviorJsonValue> { return id; }

  page(page: BehaviorPage): number {
    return page.offset;
  }

  async list(page: BehaviorPage): Promise<BehaviorJsonValue> {
    return this.behaviors.repository(Product).find(page);
  }

  async remote(id: number): Promise<BehaviorJsonValue> {
    const input = { query: { id }, headers: { accept: 'application/json' } } satisfies BehaviorHttpInput;
    return input;
  }
}

class ExplicitBehaviorConsumer {
  context?: BehaviorContext;
  service?: ProductService;
  @Needs
  lifecycle!: AbortSignal;

  @OnStart
  async initialize() {
    this.context = await Behaviors.create(AppConfiguration, {
      partition: 'public-catalog', signal: this.lifecycle, timeoutMs: 5000,
    });
    this.service = Behaviors.construct(ProductService, { behaviors: this.context, label: 'Products' });
    const offset: number = this.service.page({ page: 0, size: 20, offset: 0, sort: ['id:asc'] });
    const response: BehaviorJsonValue = await this.service.remote(offset);
    void response;
  }

  @OnStop()
  dispose() {
    this.context?.dispose();
  }
}

@RequestRoute('/products')
class ProductsController {
  @GetRoute
  async getProducts() {
    const context = await Behaviors.create(AppConfiguration, {
      partition: 'public-catalog', signal: new AbortController().signal,
    });
    try {
      const service = Behaviors.construct(ProductService, { behaviors: context, label: 'Products' });
      const repository = context.repository(Product);
      await repository.save({ id: 1, label: 'Products' });
      await repository.delete(1);
      await context.evict(service, 'load', [1]);
      await context.evict(service, 'load');
      return await repository.find({ page: 0, size: 20, sort: ['id'] });
    } finally {
      context.dispose();
    }
  }
}

class NativeCacheChecks {
  static nativeCacheConstructor: typeof globalThis.Cache = globalThis.Cache;
}
async function browserCache() {
  const nativeCache = await globalThis.caches.open('browser-cache');
  const match: Response | undefined = await nativeCache.match('/products');
  void match;
}
void NativeCacheChecks;
void browserCache;
void ExplicitBehaviorConsumer;
void ProductsController;
void HttpConfiguration;

// @ts-expect-error Setup requires a static configure method with the adapter contract.
@Setup()
class MissingConfiguration {}

// @ts-expect-error Needs marks one field and takes no arguments.
Needs("missing");

// @ts-expect-error All explicit cache resource bounds are required.
CacheSettings({ name: 'products', ttlMs: 1000 });
// @ts-expect-error The retired generic HTTP annotation is not available.
WebCall({ client: 'catalog', method: 'TRACE', path: '/products' });
// @ts-expect-error Page defaults need a default size, a maximum size and a sort list.
PageDefaults({ defaultSize: 20 });
// @ts-expect-error A Store lists fields of its Entity class.
Store(Product, { fields: ['missing'], defaultSize: 20, maxSize: 100 });
// @ts-expect-error A Store names its Entity class, not a string.
Store('Product', { fields: ['id'], defaultSize: 20, maxSize: 100 });
// @ts-expect-error Cache mutations match parameters by name, so they take no index list.
ClearCache({ method: 'load', arguments: [0] });
// @ts-expect-error A mapping entry names its target.
Convert({ fields: [{ source: 'id' }] });
Gateway({ class: 'server/services/ReportService.java' });
Gateway({ class: 'server/services/ReportService.java', method: 'summarize' });
Gateway({ command: ['ruby', 'server/helpers/report.rb'] });
// @ts-expect-error A gateway command names at least its program.
Gateway({ command: [] });
// @ts-expect-error A gateway takes one target: a class or a command.
Gateway({ method: 'summarize' });
// @ts-expect-error Entity adapter names are not arbitrary adapter objects.
Entity({ adapter: Adapters.entityAdapter, id: 'id' });

class InvalidMethods {
  // @ts-expect-error Cached only supports async JSON-returning methods.
  @Cached
  sync(id: number) { return id; }

  // @ts-expect-error GetRequest only supports async JSON-returning methods.
  @GetRequest("/products")
  date() { return new Date(); }

  // @ts-expect-error Parameter annotations cannot decorate fields.
  @PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})
  field = 1;
}

function constructorAndBoundaryChecks(context: BehaviorContext) {
  // @ts-expect-error Factories retain source field types.
  Behaviors.construct(ProductService, { behaviors: context, label: 1 });
  // @ts-expect-error Factories reject nonexistent fields.
  Behaviors.construct(ProductService, { behaviors: context, extra: 'unknown' });
  // @ts-expect-error Methods are not generated constructor fields.
  Behaviors.construct(ProductService, { load: async () => null });
  // @ts-expect-error Decorators cannot augment TypeScript constructor signatures; use construct.
  new ProductService(context, 'Products');
  // @ts-expect-error Store sorting accepts an array, never an unvalidated scalar.
  context.repository(Product).find({ sort: 'id' });
  // @ts-expect-error Store values cross a JSON record boundary.
  context.repository(Product).save({ created: new Date() });
  // @ts-expect-error Required execution authority cannot be omitted.
  Behaviors.create(AppConfiguration, { partition: 'public' });
}
void constructorAndBoundaryChecks;

class InvalidAdapters {
  static deniedConfiguration: BehaviorConfiguration = {
    // @ts-expect-error Authorization must explicitly return boolean; void never grants authority.
    authorize() {},
  };
  static brokenCache: CacheAdapter = {
    // @ts-expect-error Cache hits require an explicit finite expiration at runtime.
    get() { return { hit: true, value: null }; },
    set() {},
    delete() {},
  };
}
void InvalidAdapters;

@Converter()
class ProductMapper {
  @Convert({ fields: [{ source: 'detail.id', target: 'productId' }, { target: 'kind', constant: 'product' }] })
  async toSummary(product: BehaviorJsonValue): Promise<BehaviorJsonValue> { void product; return {}; }
}
void ProductMapper;

@Store(Product, {"fields":["id","label"],"defaultSize":20,"maxSize":100})
abstract class DeclaredRepository {
  abstract findByStatus(status: BehaviorJsonValue): Promise<BehaviorJsonValue>;
}
void DeclaredRepository;

@TestSetup
class CatalogTestConfiguration {
  static configure(_execution: BehaviorExecution): BehaviorConfiguration {
    return {};
  }
}

@TestSetup()
class AsyncTestConfiguration {
  static async configure(): Promise<BehaviorConfiguration> {
    return {};
  }
}

@Test(CatalogTestConfiguration, {"partition":"tests"})
@UseProfiles('test', 'local')
@TestSettings({"tac.catalog.size":5,"tac.catalog.name":"Sample","tac.catalog.enabled":true,"tac.catalog.optional":null})
@FreshStart
class CatalogServiceTest {
  @Fake
  catalogClient!: ProductsClient;
  @Fake()
  alternateClient!: ProductsClient;
}

@Test(AsyncTestConfiguration)
@UseProfiles('test')
@TestSettings({"yon.catalog.size":5})
@FreshStart()
class YonServiceTest {
  @Fake()
  catalogClient!: ProductsClient;
  @Fake
  alternateClient!: ProductsClient;

  @ValueSource([1, 2, 3])
  testCountsEveryInput(count: number): void {
    Assert.isTrue(count > 0, 'counts from one');
  }

  async testRefusesAnEmptyName(): Promise<void> {
    Assert.equals('Sample', 'Sample');
    await Assert.throws(async () => Promise.reject(new Error('empty name')));
  }
}

// @ts-expect-error Test configurations require a static configure method.
@TestSetup
class MissingTestConfiguration {}

// @ts-expect-error A test names its configuration.
Test({ partition: 'tests' });
// @ts-expect-error A test names its Setup class directly, not inside an object.
Test({ configuration: CatalogTestConfiguration });
// @ts-expect-error A test names its Setup class, not a string.
Test('CatalogTestConfiguration');
// @ts-expect-error A partition is a string.
Test(CatalogTestConfiguration, { partition: 1 });
// @ts-expect-error A partition is a string.
Test({ configuration: 'CatalogTestConfiguration', partition: 1 });
// @ts-expect-error Only configuration and partition options exist.
Test({ configuration: 'CatalogTestConfiguration', timeout: 100 });
// @ts-expect-error Test configuration markers take no options.
TestSetup({});
// @ts-expect-error Profiles are literal strings.
UseProfiles(1);
// @ts-expect-error At least one profile is required.
UseProfiles();
// @ts-expect-error Property paths are in a framework namespace.
TestSettings({ 'catalog.size': 5 });
// @ts-expect-error Test property values are scalars, not nested objects.
TestSettings({ 'tac.catalog': { size: 5 } });
// @ts-expect-error Undefined is not a JSON scalar.
TestSettings({ 'tac.catalog': undefined });
// @ts-expect-error FreshStart takes no options.
FreshStart('always');
// @ts-expect-error Fake takes no provider argument.
Fake('ProductsClient');
// @ts-expect-error Fake takes no provider argument.
Fake('ProductsClient');

void CatalogServiceTest;
void YonServiceTest;
