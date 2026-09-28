@Setup
@WhenProfile('test', 'development')
@WhenSetting({"name":"TAC_TEST_ENABLED","havingValue":"true","matchIfMissing":false})
class ApplicationAdaptersSetup {
  static configured = 0;
  static configure() {
    ApplicationAdaptersSetup.configured++;
    const entries = new Map();
    return {
      authorize: () => true,
      authorizeEntity: (command, execution) => command.entity === 'ProductEntity' && execution.partition === 'public-test',
      caches: {
        products: {
          get: (key) => entries.get(key) ?? { hit: false },
          set: (key, value, policy) => entries.set(key, { hit: true, value, expiresAt: policy.expiresAt }),
          delete: (key) => { entries.delete(key); },
        },
      },
      entities: {
        data: { execute: (operation) => ApplicationAdaptersSetup.executeData(operation) },
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

