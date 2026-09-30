[Service]
public sealed class CatalogService {
    public BehaviorContext behaviors;
    public long Offset = 0;

    public System.Threading.Tasks.Task<BehaviorJSON> Page([PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON input) {
        return System.Threading.Tasks.Task.FromResult(input.Get("offset"));
    }

    [OnStart]
    public async System.Threading.Tasks.Task Initialize() {
        behaviors = Behaviors.create(typeof(ApplicationSetup), "public-catalog", new BehaviorSignal());
        BehaviorJSON first = await Page(BehaviorJSON.Parse("{\"page\":1}"));
        Offset = first.Integer;
    }
}
