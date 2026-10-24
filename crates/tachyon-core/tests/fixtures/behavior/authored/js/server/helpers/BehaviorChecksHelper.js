@Helper()
class BehaviorChecksHelper {
static _VALIDATION = 'validation';
static _CONFIGURATION_PROPERTIES = 'configuration-properties';
static _INACTIVE_CONFIGURATION = 'inactive-configuration';
static _PARTITION = 'public-test';
static _QUERY_FAILED = 'query failed';
static _LIMIT = 12;
static _LIMIT_TEXT = '12';
static _MANIFEST_LIMIT = 7;
static _PRODUCT_ID = 7;
static _PRICE = 5;
static _REPLACED_CALLS = 99;
static _RELOADED_CALLS = 3;
static _PAGE = 2;
static _PAGE_SIZE = 5;

static _thrownErrorCode(action) {
  try {
    action();
  } catch (error) {
    return error.code;
  }
  return undefined;
}

static async _thrownErrorCodeAsync(action) {
  try {
    await action();
  } catch (error) {
    return error.code;
  }
  return undefined;
}

static _validationFailurePath(action) {
  try {
    action();
  } catch (error) {
    if (error.code !== BehaviorChecksHelper._VALIDATION) throw error;
    return error.path;
  }
  return undefined;
}

static async _validationFailurePathAsync(action) {
  try {
    await action();
  } catch (error) {
    if (error.code !== BehaviorChecksHelper._VALIDATION) throw error;
    return error.path;
  }
  return undefined;
}

static _checkUnknownPropertyIsRejected(supplied) {
  const withTypo = { ...supplied, TAC_TEST_TYPO: 'invalid' };
  const propertyError = BehaviorChecksHelper._thrownErrorCode(() => Behaviors.properties(ApplicationSettings, withTypo));
  if (propertyError !== BehaviorChecksHelper._CONFIGURATION_PROPERTIES) throw new Error('unknown configuration property accepted');
  return propertyError;
}

static _checkApplicationValues() {
  const application = Behaviors.values(ApplicationValues, { tac: { test: { limit: BehaviorChecksHelper._LIMIT, enabled: true, label: 'Products' } } });
  const bound = application.limitText === BehaviorChecksHelper._LIMIT_TEXT && application.limit === BehaviorChecksHelper._LIMIT && application.missing === 'fallback';
  if (!bound) throw new Error('application values were not bound');
  const valueError = BehaviorChecksHelper._thrownErrorCode(() => Behaviors.values(ApplicationValues, { secrets: {} }));
  if (valueError !== BehaviorChecksHelper._CONFIGURATION_PROPERTIES) throw new Error('unknown application section accepted');
  const manifest = Behaviors.values(ApplicationValues, { name: 'Catalog', icons: [{ src: '/icon.svg' }], tac: { test: { limit: BehaviorChecksHelper._MANIFEST_LIMIT } } });
  if (manifest.limit !== BehaviorChecksHelper._MANIFEST_LIMIT) throw new Error('manifest members were not ignored');
}

static async _checkInactiveConfiguration(signal, configuredBefore) {
  const inactive = await BehaviorChecksHelper._thrownErrorCodeAsync(() => Behaviors.create(ApplicationAdaptersSetup, { partition: BehaviorChecksHelper._PARTITION, signal }));
  const executed = ApplicationAdaptersSetup.configured !== configuredBefore;
  if (inactive !== BehaviorChecksHelper._INACTIVE_CONFIGURATION || executed) throw new Error('inactive configuration executed');
  return inactive;
}

static async _checkUnselectedConfigurations(signal, supplied, configuredBefore) {
  const selections = [
    { profiles: ['production'], properties: supplied },
    { profiles: ['development'], properties: { ...supplied, TAC_TEST_ENABLED: 'false' } },
  ];
  const selectionErrors = [];
  for (const options of selections) {
    const code = await BehaviorChecksHelper._thrownErrorCodeAsync(() => Behaviors.create(ApplicationAdaptersSetup, { partition: BehaviorChecksHelper._PARTITION, signal, ...options }));
    const executed = ApplicationAdaptersSetup.configured !== configuredBefore;
    if (code !== BehaviorChecksHelper._INACTIVE_CONFIGURATION || executed) throw new Error('configuration condition did not execute');
    selectionErrors.push(code);
  }
  return selectionErrors;
}

static _checkConstraints(valid) {
  const cases = [['id', null, 'id.notNull'], ['name', 'a', 'name.size'], ['price', -1, 'price.min'], ['price', 11, 'price.max']];
  const constraintPaths = [];
  for (const [field, value, expected] of cases) {
    const rejectedPath = BehaviorChecksHelper._validationFailurePath(() => Behaviors.validate(ProductInput, { ...valid, [field]: value }));
    if (rejectedPath !== expected) throw new Error('DTO constraint did not execute');
    constraintPaths.push(rejectedPath);
  }
  return constraintPaths;
}

static async _exerciseValidation() {
  const input = { id: BehaviorChecksHelper._PRODUCT_ID, name: 'é', price: BehaviorChecksHelper._PRICE, extra: true };
  const copied = Behaviors.validate(ProductInput, input);
  input.name = 'Changed after validation';
  const service = new InputService();
  const validated = await service.submit(copied);
  const rejected = service.submit({ id: BehaviorChecksHelper._PRODUCT_ID, name: '\u00a0', price: BehaviorChecksHelper._PRICE });
  if (!(rejected instanceof Promise)) throw new Error('Valid changed the async method contract');
  const validationPath = await BehaviorChecksHelper._validationFailurePathAsync(() => rejected);
  if (validationPath !== 'name.notBlank' || service.calls !== 1) throw new Error('invalid DTO reached application code');
  const constraintPaths = BehaviorChecksHelper._checkConstraints(copied);
  return { input: validated, path: validationPath, constraintPaths, calls: service.calls, copyPreserved: copied.name === 'é' };
}

static async _exerciseCache(catalog, context) {
  const first = await catalog.find(BehaviorChecksHelper._PRODUCT_ID);
  const cached = await catalog.find(BehaviorChecksHelper._PRODUCT_ID);
  await context.evict(catalog, 'find', [BehaviorChecksHelper._PRODUCT_ID]);
  const fresh = await catalog.find(BehaviorChecksHelper._PRODUCT_ID);
  await catalog.replace(BehaviorChecksHelper._PRODUCT_ID, BehaviorChecksHelper._REPLACED_CALLS);
  const replaced = await catalog.find(BehaviorChecksHelper._PRODUCT_ID);
  if (replaced.calls !== BehaviorChecksHelper._REPLACED_CALLS) throw new Error('cache put failed');
  await catalog.remove(BehaviorChecksHelper._PRODUCT_ID);
  const reloaded = await catalog.find(BehaviorChecksHelper._PRODUCT_ID);
  if (reloaded.calls !== BehaviorChecksHelper._RELOADED_CALLS) throw new Error('cache evict failed');
  return { first, cached, fresh };
}

static async _checkCacheKey(catalog) {
  await catalog.price('sku-1', 'first');
  await catalog.price('sku-1', 'second');
  if (catalog.prices !== 1) throw new Error('CacheKey hit ignores the unmarked argument');
  await catalog.price('sku-2', 'first');
  if (catalog.prices !== 2) throw new Error('CacheKey miss on a different key');
}

static async _checkQueries(context) {
  const queries = new ProductQueriesStore();
  queries.behaviors = context;
  const found = await queries.findById(BehaviorChecksHelper._PRODUCT_ID);
  if (found.id !== BehaviorChecksHelper._PRODUCT_ID) throw new Error(BehaviorChecksHelper._QUERY_FAILED);
  const all = await queries.findAll({});
  if (all.length !== 1) throw new Error(BehaviorChecksHelper._QUERY_FAILED);
  const saved = await queries.save({ id: BehaviorChecksHelper._PRODUCT_ID });
  if (saved.operation !== 'save') throw new Error(BehaviorChecksHelper._QUERY_FAILED);
  const deleted = await queries.deleteById(BehaviorChecksHelper._PRODUCT_ID);
  if (deleted !== 1) throw new Error(BehaviorChecksHelper._QUERY_FAILED);
  const count = await queries.count_by_id(BehaviorChecksHelper._PRODUCT_ID);
  if (count !== 1) throw new Error(BehaviorChecksHelper._QUERY_FAILED);
  const available = await queries.available(BehaviorChecksHelper._PRODUCT_ID);
  if (available !== true) throw new Error(BehaviorChecksHelper._QUERY_FAILED);
}

static async exerciseBehaviors(signal) {
  const supplied = { TAC_TEST_LABEL: 'Products', TAC_TEST_ENABLED: 'true', TAC_TEST_LIMIT: BehaviorChecksHelper._LIMIT_TEXT };
  const settings = Behaviors.properties(ApplicationSettings, supplied);
  supplied.TAC_TEST_LABEL = 'Changed after binding';
  const propertyError = BehaviorChecksHelper._checkUnknownPropertyIsRejected(supplied);
  BehaviorChecksHelper._checkApplicationValues();
  const configuredBefore = ApplicationAdaptersSetup.configured;
  const inactive = await BehaviorChecksHelper._checkInactiveConfiguration(signal, configuredBefore);
  const selectionErrors = await BehaviorChecksHelper._checkUnselectedConfigurations(signal, supplied, configuredBefore);
  const context = await Behaviors.create(ApplicationAdaptersSetup, { partition: BehaviorChecksHelper._PARTITION, signal, profiles: ['development'], properties: supplied });
  try {
    const validation = await BehaviorChecksHelper._exerciseValidation();
    const catalog = Object.assign(new CatalogHelper(), { behaviors: context });
    const client = Object.assign(new CatalogClient(), { behaviors: context });
    const { first, cached, fresh } = await BehaviorChecksHelper._exerciseCache(catalog, context);
    await BehaviorChecksHelper._checkCacheKey(catalog);
    await BehaviorChecksHelper._checkQueries(context);
    const page = catalog.page({ page: BehaviorChecksHelper._PAGE, size: BehaviorChecksHelper._PAGE_SIZE, sort: ['id'] });
    const repository = context.repository(ProductEntity);
    const calls = ApplicationAdaptersSetup.configured - configuredBefore;
    const located = await client.lookup(BehaviorChecksHelper._PRODUCT_ID);
    if (located.path !== '/products/7') throw new Error('path variable');
    return {
      first, cached, fresh, page,
      configuration: { settings, inactive, selectionErrors, propertyError, calls },
      validation,
      remote: await client.remote(),
      declared: await client.search(BehaviorChecksHelper._PAGE, 'acme'),
      saved: await repository.save({ id: 1 }),
      found: await repository.find({ page: 0, size: BehaviorChecksHelper._PAGE_SIZE, sort: [] }),
      deleted: await repository.delete(1),
    };
  } finally {
    context.dispose();
  }
}
}
