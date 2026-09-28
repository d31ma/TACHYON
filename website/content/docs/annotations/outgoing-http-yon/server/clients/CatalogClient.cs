[HTTPClient]
class CatalogClient {
    public BehaviorContext behaviors;

    [GetRequest("/products")]
    public async System.Threading.Tasks.Task<BehaviorJSON> All([RequestParam("page")] BehaviorJSON page) {}

    [PostRequest("/products")]
    public async System.Threading.Tasks.Task<BehaviorJSON> Add([RequestBody] BehaviorJSON product) {}

    [GetRequest("/products")]
    public async System.Threading.Tasks.Task<BehaviorJSON> First() {
        await System.Threading.Tasks.Task.CompletedTask;
        return BehaviorJSON.Parse("{\"query\":{\"page\":1}}");
    }

    [OnStart]
    public void Initialize() {
        behaviors = Behaviors.create(typeof(CatalogSetup), "catalog", new BehaviorSignal());
    }
}
