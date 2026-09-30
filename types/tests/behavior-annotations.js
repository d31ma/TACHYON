@Setup()
class Settings {
  /** @param {BehaviorExecution} execution */
  static configure(execution) {
    /** @type {BehaviorConfiguration} */
    const configuration = {
      http: {
        catalog: {
          async exchange(request) {
            return { path: request.path, owner: execution.partition };
          },
        },
      },
    };
    return configuration;
  }
}

@CacheSettings({"name":"products","ttlMs":1000,"maxBytes":4096,"maxEntries":20})
class Catalog {
  /** @param {BehaviorContext} behaviors */
  constructor(behaviors) { this.behaviors = behaviors; }

  /** @param {number} id */
  @Cached
  async load(id) { return { id }; }

  /** @returns {Promise<BehaviorJsonValue>} */
  async remote() { return { query: { page: 0 } }; }
}

@HTTPClient({"adapter":"catalog"})
class CatalogRemoteClient {
  /** @returns {Promise<BehaviorJsonValue | void>} */
  @GetRequest("/products")
  async remote() {}
}
void CatalogRemoteClient;

async function explicitOwnership() {
  const context = await Behaviors.create(Settings, {
    partition: 'public-catalog', signal: new AbortController().signal,
  });
  try {
    return await new Catalog(context).remote();
  } finally {
    context.dispose();
  }
}
void explicitOwnership;

@Entity({"adapter":"data"})
@Table("orders")
class Order {
  @Id
  id = 0;
}

/** @extends {Repository<Order, number>} */
@Store(Order, {"fields":["id"],"defaultSize":20,"maxSize":100})
class OrderStore extends Repository {}

/** @param {OrderStore} orders */
async function useOrderStore(orders) {
  /** @type {Order | null} */
  const found = await orders.findById(1);
  /** @type {number} */
  const deleted = await orders.deleteById(1);
  return [found, deleted];
}
void useOrderStore;
