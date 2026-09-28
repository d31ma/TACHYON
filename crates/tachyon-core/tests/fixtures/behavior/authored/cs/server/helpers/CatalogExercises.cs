[Description("Behavior conformance CatalogExercises")]
class CatalogExercises {
    internal static async System.Threading.Tasks.Task<BehaviorJSON> run() {
        ConfigurationAnnotationTests.Run();
        await ValidationAnnotationTests.Run();
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(AnnotatedConfigSetup), "tenant-one", signal);
        var catalog = new AnnotatedCatalogClient { behaviors = context };
        var input = BehaviorJSON.Parse("{\"page\":2,\"size\":3}");
        var first = await catalog.list(input); var cached = await catalog.list(input);
        await context.evict("AnnotatedCatalogClient", "list", input); var fresh = await catalog.list(input);
        if ((await catalog.find(BehaviorJSON.From(7))).Get("path").Text != "/products/7") throw new System.Exception("path variable");
        var repository = context.repository(typeof(AnnotatedProductEntity));
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> {
            ["first"] = first, ["cached"] = cached, ["fresh"] = fresh,
            ["remote"] = await catalog.remote(), ["page"] = catalog.page(input),
            ["saved"] = await repository.save(BehaviorJSON.Parse("{\"id\":4}")),
            ["found"] = await repository.find(input), ["deleted"] = await repository.delete(BehaviorJSON.From(4))
        });
    }
}
