@HTTPClient
class CatalogClient {
  behaviors!: BehaviorContext;

  @GetRequest("/products")
  async all(@RequestParam("page") page: number): Promise<BehaviorJsonValue> {}

  @PostRequest("/products")
  async add(@RequestBody product: BehaviorJsonValue): Promise<BehaviorJsonValue> {}

  @GetRequest("/products")
  async first(): Promise<BehaviorJsonValue> { return { query: { page: 1 } }; }

  @OnStart
  async initialize(): Promise<void> {
    this.behaviors = await Behaviors.create(CatalogSetup, {
      partition: 'catalog',
      signal: AbortSignal.timeout(10000),
    });
  }
}
