[Description("Behavior conformance CatalogExercises")]
class CatalogExercises {
    internal static async System.Threading.Tasks.Task<BehaviorJSON> run() {
        ConfigurationAnnotationTests.Run();
        await ValidationAnnotationTests.Run();
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(AnnotatedConfigSetup), "tenant-one", signal);
        var catalog = new AnnotatedCatalogService { behaviors = context };
        var client = new AnnotatedCatalogClient { behaviors = context };
        var input = BehaviorJSON.Parse("{\"page\":2,\"size\":3}");
        var first = await catalog.list(input); var cached = await catalog.list(input);
        await context.evict("AnnotatedCatalogService", "list", input); var fresh = await catalog.list(input);
        if ((await client.find(BehaviorJSON.From(7))).Get("path").Text != "/products/7") throw new System.Exception("path variable");
        await CheckCacheKey(catalog);
        await CheckRepositoryStore(context, input);
        var repository = context.repository(typeof(AnnotatedProductEntity));
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["first"] = first, ["cached"] = cached, ["fresh"] = fresh,
            ["remote"] = await client.remote(), ["page"] = catalog.page(input),
            ["saved"] = await repository.save(BehaviorJSON.Parse("{\"id\":4}")),
            ["found"] = await repository.find(input), ["deleted"] = await repository.delete(BehaviorJSON.From(4))
        });
    }
    private static async System.Threading.Tasks.Task CheckCacheKey(AnnotatedCatalogService catalog) {
        await catalog.price(BehaviorJSON.From("sku-1"), BehaviorJSON.From("first"));
        await catalog.price(BehaviorJSON.From("sku-1"), BehaviorJSON.From("second"));
        if (catalog.priceCalls != 1 || Config.LastArguments.Items.Length != 1) throw new System.Exception("CacheKey hit ignores the unmarked argument");
        await catalog.price(BehaviorJSON.From("sku-2"), BehaviorJSON.From("first"));
        if (catalog.priceCalls != 2) throw new System.Exception("CacheKey miss on a different key");
    }
    private static async System.Threading.Tasks.Task CheckRepositoryStore(BehaviorContext context, BehaviorJSON page) {
        var queries = new AnnotatedQueriesStore { behaviors = context };
        if ((await queries.findById(1)).Get("id").Integer != 1) throw new System.Exception("repository findById");
        if ((await queries.findAll(page)).Items.Length != 1) throw new System.Exception("repository findAll");
        if ((await queries.save(BehaviorJSON.Parse("{\"id\":4}"))).Get("operation").Text != "save") throw new System.Exception("repository save");
        if ((await queries.deleteById(4)).Integer != 1) throw new System.Exception("repository deleteById");
        if ((await queries.deleteByName(BehaviorJSON.From("old"))).Integer != 1) throw new System.Exception("derived delete");
    }
}
