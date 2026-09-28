[Service]
[HTTPClient]
public class CatalogService {
    public BehaviorContext behaviors;
    public int count = 0;

    [OnStart]
    public void Initialize() {
        behaviors = Behaviors.create(typeof(CatalogSetup), "catalog", new BehaviorSignal());
    }

    [GetRequest("/api/products")]
    public async System.Threading.Tasks.Task<BehaviorJSON> All() { return BehaviorJSON.Parse("{}"); }
}
