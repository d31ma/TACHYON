@Settings({"prefix":"TAC_TEST_"})
class ApplicationSettings {
    /** @type {string} */
    label;
    enabled = false;
    @Min(1)
    @Max(100)
    limit = 20;
    @Min(0)
    @Max(10)
    ratio = 1.5;
}

class ApplicationValues {
  @Setting("${tac.test.limit:5}")
  limitText;
  @Setting({"path":"tac.test.limit","type":"integer","default":5,"min":1,"max":100})
  limit;
  @Setting("${tac.test.missing:fallback}")
  missing;
}

@Setup
@WhenProfile('test', 'development')
@WhenSetting({"name":"TAC_TEST_ENABLED","havingValue":"true","matchIfMissing":false})
class ApplicationAdapters {
  static configured = 0;
  static configure() {
    ApplicationAdapters.configured++;
    const entries = new Map();
    return {
      authorize: () => true,
      authorizeEntity: (command, execution) => command.entity === 'Product' && execution.partition === 'public-test',
      caches: {
        products: {
          get: (key) => entries.get(key) ?? { hit: false },
          set: (key, value, policy) => entries.set(key, { hit: true, value, expiresAt: policy.expiresAt }),
          delete: (key) => { entries.delete(key); },
        },
      },
      entities: {
        data: { execute: (operation) => ApplicationAdapters.executeData(operation) },
      },
      http: {
        catalog: { exchange: (request) => ({ method: request.method, path: request.path, query: request.query, headers: request.headers }) },
      },
    };
  }

  static executeData(operation) {
    if (operation.operation !== 'query') {
      return { operation: operation.operation, table: operation.table };
    }
    if (operation.query.kind === 'count') return 1;
    if (operation.query.kind === 'exists') return true;
    return [{ id: operation.query.where[0].value }];
  }
}

@Checked
class ProductInput {
  @Required
  id;
  @NotEmpty
  @Length({"min":2,"max":8})
  name;
  @Min(0)
  @Max(10)
  price;
}

class InputService {
  calls = 0;
  /** @param {ProductInput} input */
  async submit(@Check input) {
    this.calls++;
    return { ...input, validated: true };
  }
}

@Entity({"adapter":"data"})
@Table('products')
class Product {
  @Id
  id = 1;
}

@Store(Product, {"fields":["id"],"defaultSize":20,"maxSize":100})
class ProductQueries {
  @Needs
  behaviors;
  async findById(id) {}
  async count_by_id(id) {}
  @Query({"operation":"exists","where":[{"field":"id","operator":"eq","argument":0}],"orderBy":[]})
  async available(id) {}
}

@CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})
class Catalog {
  @Needs
  behaviors;
  calls = 0;

  @Cached
  async find(id) {
    this.calls++;
    return { id, calls: this.calls };
  }
  @UpdateCache({"method":"find"})
  async replace(id, calls) { return { id, calls }; }
  @ClearCache({"method":"find"})
  async remove(id) { return true; }

  page(@PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]}) input) { return input; }

  @WebCall({"client":"catalog","method":"GET","path":"/products"})
  async remote() { return {}; }

  @WebCall({"client":"catalog","method":"GET","path":"/products","query":{"page":0},"headers":{"X-Tenant":1}})
  async search(page, tenant) {}

  @Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})
  @MaxAtOnce({"limit":2,"policy":"block"})
  @AllOrNothing({"adapter":"catalog"})
  async flaky() {
    this.calls++;
    if (this.calls < 3) throw Error('transient');
    return this.calls;
  }
}

function thrownErrorCode(action) {
  try {
    action();
  } catch (error) {
    return error.code;
  }
  return undefined;
}

async function thrownErrorCodeAsync(action) {
  try {
    await action();
  } catch (error) {
    return error.code;
  }
  return undefined;
}

function validationFailurePath(action) {
  try {
    action();
  } catch (error) {
    if (error.code !== 'validation') throw error;
    return error.path;
  }
  return undefined;
}

async function validationFailurePathAsync(action) {
  try {
    await action();
  } catch (error) {
    if (error.code !== 'validation') throw error;
    return error.path;
  }
  return undefined;
}

function checkUnknownPropertyIsRejected(supplied) {
  const withTypo = { ...supplied, TAC_TEST_TYPO: 'invalid' };
  const propertyError = thrownErrorCode(() => Behaviors.properties(ApplicationSettings, withTypo));
  if (propertyError !== 'configuration-properties') throw new Error('unknown configuration property accepted');
  return propertyError;
}

function checkApplicationValues() {
  const application = Behaviors.values(ApplicationValues, { tac: { test: { limit: 12, enabled: true, label: 'Products' } } });
  const bound = application.limitText === '12' && application.limit === 12 && application.missing === 'fallback';
  if (!bound) throw new Error('application values were not bound');
  const valueError = thrownErrorCode(() => Behaviors.values(ApplicationValues, { secrets: {} }));
  if (valueError !== 'configuration-properties') throw new Error('unknown application section accepted');
  const manifest = Behaviors.values(ApplicationValues, { name: 'Catalog', icons: [{ src: '/icon.svg' }], tac: { test: { limit: 7 } } });
  if (manifest.limit !== 7) throw new Error('manifest members were not ignored');
}

async function checkInactiveConfiguration(signal, configuredBefore) {
  const inactive = await thrownErrorCodeAsync(() => Behaviors.create(ApplicationAdapters, { partition: 'public-test', signal }));
  const executed = ApplicationAdapters.configured !== configuredBefore;
  if (inactive !== 'inactive-configuration' || executed) throw new Error('inactive configuration executed');
  return inactive;
}

async function checkUnselectedConfigurations(signal, supplied, configuredBefore) {
  const selections = [
    { profiles: ['production'], properties: supplied },
    { profiles: ['development'], properties: { ...supplied, TAC_TEST_ENABLED: 'false' } },
  ];
  const selectionErrors = [];
  for (const options of selections) {
    const code = await thrownErrorCodeAsync(() => Behaviors.create(ApplicationAdapters, { partition: 'public-test', signal, ...options }));
    const executed = ApplicationAdapters.configured !== configuredBefore;
    if (code !== 'inactive-configuration' || executed) throw new Error('configuration condition did not execute');
    selectionErrors.push(code);
  }
  return selectionErrors;
}

function checkConstraints(valid) {
  const cases = [['id', null, 'id.notNull'], ['name', 'a', 'name.size'], ['price', -1, 'price.min'], ['price', 11, 'price.max']];
  const constraintPaths = [];
  for (const [field, value, expected] of cases) {
    const rejectedPath = validationFailurePath(() => Behaviors.validate(ProductInput, { ...valid, [field]: value }));
    if (rejectedPath !== expected) throw new Error('DTO constraint did not execute');
    constraintPaths.push(rejectedPath);
  }
  return constraintPaths;
}

async function exerciseValidation() {
  const input = { id: 7, name: 'é', price: 5, extra: true };
  const copied = Behaviors.validate(ProductInput, input);
  input.name = 'Changed after validation';
  const service = new InputService();
  const validated = await service.submit(copied);
  const rejected = service.submit({ id: 7, name: '\u00a0', price: 5 });
  if (!(rejected instanceof Promise)) throw new Error('Valid changed the async method contract');
  const validationPath = await validationFailurePathAsync(() => rejected);
  if (validationPath !== 'name.notBlank' || service.calls !== 1) throw new Error('invalid DTO reached application code');
  const constraintPaths = checkConstraints(copied);
  return { input: validated, path: validationPath, constraintPaths, calls: service.calls, copyPreserved: copied.name === 'é' };
}

async function exerciseCache(catalog, context) {
  const first = await catalog.find(7);
  const cached = await catalog.find(7);
  await context.evict(catalog, 'find', [7]);
  const fresh = await catalog.find(7);
  await catalog.replace(7, 99);
  const replaced = await catalog.find(7);
  if (replaced.calls !== 99) throw new Error('cache put failed');
  await catalog.remove(7);
  const reloaded = await catalog.find(7);
  if (reloaded.calls !== 3) throw new Error('cache evict failed');
  return { first, cached, fresh };
}

async function checkQueries(context) {
  const queries = Behaviors.construct(ProductQueries, { behaviors: context });
  const rows = await queries.findById(7);
  if (rows[0].id !== 7) throw new Error('query failed');
  const count = await queries.count_by_id(7);
  if (count !== 1) throw new Error('query failed');
  const available = await queries.available(7);
  if (available !== true) throw new Error('query failed');
}

async function exerciseBehaviors(signal) {
  const supplied = { TAC_TEST_LABEL: 'Products', TAC_TEST_ENABLED: 'true', TAC_TEST_LIMIT: '12' };
  const settings = Behaviors.properties(ApplicationSettings, supplied);
  supplied.TAC_TEST_LABEL = 'Changed after binding';
  const propertyError = checkUnknownPropertyIsRejected(supplied);
  checkApplicationValues();
  const configuredBefore = ApplicationAdapters.configured;
  const inactive = await checkInactiveConfiguration(signal, configuredBefore);
  const selectionErrors = await checkUnselectedConfigurations(signal, supplied, configuredBefore);
  const context = await Behaviors.create(ApplicationAdapters, { partition: 'public-test', signal, profiles: ['development'], properties: supplied });
  try {
    const validation = await exerciseValidation();
    const catalog = Behaviors.construct(Catalog, { behaviors: context });
    const { first, cached, fresh } = await exerciseCache(catalog, context);
    await checkQueries(context);
    const page = catalog.page({ page: 2, size: 5, sort: ['id'] });
    const repository = context.repository(Product);
    const calls = ApplicationAdapters.configured - configuredBefore;
    return {
      first, cached, fresh, page,
      configuration: { settings, inactive, selectionErrors, propertyError, calls },
      validation,
      remote: await catalog.remote(),
      declared: await catalog.search(2, 'acme'),
      saved: await repository.save({ id: 1 }),
      found: await repository.find({ page: 0, size: 5, sort: [] }),
      deleted: await repository.delete(1),
    };
  } finally {
    context.dispose();
  }
}
