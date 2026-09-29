@HTTPClient
export default class CatalogClient {
  /** @type {BehaviorContext} */
  behaviors;

  /** @param {number} page */
  @GetRequest("/products")
  async all(@RequestParam("page") page) {}

  /** @param {object} product */
  @PostRequest("/products")
  async add(@RequestBody product) {}

  /** @returns {Promise<{ query: { page: number } }>} */
  @GetRequest("/products")
  async first() { return { query: { page: 1 } }; }

  @OnStart
  async initialize() {
    this.behaviors = await Behaviors.create(CatalogSetup, {
      partition: 'catalog',
      signal: AbortSignal.timeout(10000),
    });
  }
}
