@Helper()
class BehaviorChecksHelper {
static VALIDATION = 'validation';
static CONFIGURATION_PROPERTIES = 'configuration-properties';
static INACTIVE_CONFIGURATION = 'inactive-configuration';
static PARTITION = 'public-test';
static QUERY_FAILED = 'query failed';
static LIMIT = 12;
static LIMIT_TEXT = '12';
static MANIFEST_LIMIT = 7;
static PRODUCT_ID = 7;
static PRICE = 5;
static REPLACED_CALLS = 99;
static RELOADED_CALLS = 3;
static PAGE = 2;
static PAGE_SIZE = 5;

static thrownErrorCode(action) {
  try {
    action();
  } catch (error) {
    return error.code;
  }
  return undefined;
}

static async thrownErrorCodeAsync(action) {
  try {
    await action();
  } catch (error) {
    return error.code;
  }
  return undefined;
}

static validationFailurePath(action) {
  try {
    action();
  } catch (error) {
    if (error.code !== BehaviorChecksHelper.VALIDATION) throw error;
    return error.path;
  }
  return undefined;
}

static async validationFailurePathAsync(action) {
  try {
    await action();
  } catch (error) {
    if (error.code !== BehaviorChecksHelper.VALIDATION) throw error;
    return error.path;
  }
  return undefined;
}

static checkUnknownPropertyIsRejected(supplied) {
  const withTypo = { ...supplied, TAC_TEST_TYPO: 'invalid' };
  const propertyError = BehaviorChecksHelper.thrownErrorCode(() => Behaviors.properties(ApplicationSettings, withTypo));
  if (propertyError !== BehaviorChecksHelper.CONFIGURATION_PROPERTIES) throw new Error('unknown configuration property accepted');
  return propertyError;
}

static checkApplicationValues() {
  const application = Behaviors.values(ApplicationValues, { tac: { test: { limit: BehaviorChecksHelper.LIMIT, enabled: true, label: 'Products' } } });
  const bound = application.limitText === BehaviorChecksHelper.LIMIT_TEXT && application.limit === BehaviorChecksHelper.LIMIT && application.missing === 'fallback';
  if (!bound) throw new Error('application values were not bound');
  const valueError = BehaviorChecksHelper.thrownErrorCode(() => Behaviors.values(ApplicationValues, { secrets: {} }));
  if (valueError !== BehaviorChecksHelper.CONFIGURATION_PROPERTIES) throw new Error('unknown application section accepted');
  const manifest = Behaviors.values(ApplicationValues, { name: 'Catalog', icons: [{ src: '/icon.svg' }], tac: { test: { limit: BehaviorChecksHelper.MANIFEST_LIMIT } } });
  if (manifest.limit !== BehaviorChecksHelper.MANIFEST_LIMIT) throw new Error('manifest members were not ignored');
}

static async checkInactiveConfiguration(signal, configuredBefore) {
  const inactive = await BehaviorChecksHelper.thrownErrorCodeAsync(() => Behaviors.create(ApplicationAdaptersSetup, { partition: BehaviorChecksHelper.PARTITION, signal }));
  const executed = ApplicationAdaptersSetup.configured !== configuredBefore;
  if (inactive !== BehaviorChecksHelper.INACTIVE_CONFIGURATION || executed) throw new Error('inactive configuration executed');
  return inactive;
}

static async checkUnselectedConfigurations(signal, supplied, configuredBefore) {
  const selections = [
    { profiles: ['production'], properties: supplied },
    { profiles: ['development'], properties: { ...supplied, TAC_TEST_ENABLED: 'false' } },
  ];
  const selectionErrors = [];
  for (const options of selections) {
    const code = await BehaviorChecksHelper.thrownErrorCodeAsync(() => Behaviors.create(ApplicationAdaptersSetup, { partition: BehaviorChecksHelper.PARTITION, signal, ...options }));
    const executed = ApplicationAdaptersSetup.configured !== configuredBefore;
    if (code !== BehaviorChecksHelper.INACTIVE_CONFIGURATION || executed) throw new Error('configuration condition did not execute');
    selectionErrors.push(code);
  }
  return selectionErrors;
}

static checkConstraints(valid) {
  const cases = [['id', null, 'id.notNull'], ['name', 'a', 'name.size'], ['price', -1, 'price.min'], ['price', 11, 'price.max']];
  const constraintPaths = [];
  for (const [field, value, expected] of cases) {
    const rejectedPath = BehaviorChecksHelper.validationFailurePath(() => Behaviors.validate(ProductInput, { ...valid, [field]: value }));
    if (rejectedPath !== expected) throw new Error('DTO constraint did not execute');
    constraintPaths.push(rejectedPath);
  }
  return constraintPaths;
}

static async exerciseValidation() {
  const input = { id: BehaviorChecksHelper.PRODUCT_ID, name: 'é', price: BehaviorChecksHelper.PRICE, extra: true };
  const copied = Behaviors.validate(ProductInput, input);
  input.name = 'Changed after validation';
  const service = new InputService();
  const validated = await service.submit(copied);
  const rejected = service.submit({ id: BehaviorChecksHelper.PRODUCT_ID, name: '\u00a0', price: BehaviorChecksHelper.PRICE });
  if (!(rejected instanceof Promise)) throw new Error('Valid changed the async method contract');
  const validationPath = await BehaviorChecksHelper.validationFailurePathAsync(() => rejected);
  if (validationPath !== 'name.notBlank' || service.calls !== 1) throw new Error('invalid DTO reached application code');
  const constraintPaths = BehaviorChecksHelper.checkConstraints(copied);
  return { input: validated, path: validationPath, constraintPaths, calls: service.calls, copyPreserved: copied.name === 'é' };
}

static async exerciseCache(catalog, context) {
  const first = await catalog.find(BehaviorChecksHelper.PRODUCT_ID);
  const cached = await catalog.find(BehaviorChecksHelper.PRODUCT_ID);
  await context.evict(catalog, 'find', [BehaviorChecksHelper.PRODUCT_ID]);
  const fresh = await catalog.find(BehaviorChecksHelper.PRODUCT_ID);
  await catalog.replace(BehaviorChecksHelper.PRODUCT_ID, BehaviorChecksHelper.REPLACED_CALLS);
  const replaced = await catalog.find(BehaviorChecksHelper.PRODUCT_ID);
  if (replaced.calls !== BehaviorChecksHelper.REPLACED_CALLS) throw new Error('cache put failed');
  await catalog.remove(BehaviorChecksHelper.PRODUCT_ID);
  const reloaded = await catalog.find(BehaviorChecksHelper.PRODUCT_ID);
  if (reloaded.calls !== BehaviorChecksHelper.RELOADED_CALLS) throw new Error('cache evict failed');
  return { first, cached, fresh };
}

static async checkQueries(context) {
  const queries = Behaviors.construct(ProductQueriesStore, { behaviors: context });
  const rows = await queries.findById(BehaviorChecksHelper.PRODUCT_ID);
  if (rows[0].id !== BehaviorChecksHelper.PRODUCT_ID) throw new Error(BehaviorChecksHelper.QUERY_FAILED);
  const count = await queries.count_by_id(BehaviorChecksHelper.PRODUCT_ID);
  if (count !== 1) throw new Error(BehaviorChecksHelper.QUERY_FAILED);
  const available = await queries.available(BehaviorChecksHelper.PRODUCT_ID);
  if (available !== true) throw new Error(BehaviorChecksHelper.QUERY_FAILED);
}

static async exerciseBehaviors(signal) {
  const supplied = { TAC_TEST_LABEL: 'Products', TAC_TEST_ENABLED: 'true', TAC_TEST_LIMIT: BehaviorChecksHelper.LIMIT_TEXT };
  const settings = Behaviors.properties(ApplicationSettings, supplied);
  supplied.TAC_TEST_LABEL = 'Changed after binding';
  const propertyError = BehaviorChecksHelper.checkUnknownPropertyIsRejected(supplied);
  BehaviorChecksHelper.checkApplicationValues();
  const configuredBefore = ApplicationAdaptersSetup.configured;
  const inactive = await BehaviorChecksHelper.checkInactiveConfiguration(signal, configuredBefore);
  const selectionErrors = await BehaviorChecksHelper.checkUnselectedConfigurations(signal, supplied, configuredBefore);
  const context = await Behaviors.create(ApplicationAdaptersSetup, { partition: BehaviorChecksHelper.PARTITION, signal, profiles: ['development'], properties: supplied });
  try {
    const validation = await BehaviorChecksHelper.exerciseValidation();
    const catalog = Object.assign(new CatalogClient(), { behaviors: context });
    const { first, cached, fresh } = await BehaviorChecksHelper.exerciseCache(catalog, context);
    await BehaviorChecksHelper.checkQueries(context);
    const page = catalog.page({ page: BehaviorChecksHelper.PAGE, size: BehaviorChecksHelper.PAGE_SIZE, sort: ['id'] });
    const repository = context.repository(ProductEntity);
    const calls = ApplicationAdaptersSetup.configured - configuredBefore;
    const located = await catalog.lookup(BehaviorChecksHelper.PRODUCT_ID);
    if (located.path !== '/products/7') throw new Error('path variable');
    return {
      first, cached, fresh, page,
      configuration: { settings, inactive, selectionErrors, propertyError, calls },
      validation,
      remote: await catalog.remote(),
      declared: await catalog.search(BehaviorChecksHelper.PAGE, 'acme'),
      saved: await repository.save({ id: 1 }),
      found: await repository.find({ page: 0, size: BehaviorChecksHelper.PAGE_SIZE, sort: [] }),
      deleted: await repository.delete(1),
    };
  } finally {
    context.dispose();
  }
}
}
