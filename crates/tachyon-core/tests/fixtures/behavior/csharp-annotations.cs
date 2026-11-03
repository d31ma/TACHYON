[Setup]
public sealed class AnnotatedConfig {
    public static BehaviorAdapters configure(BehaviorExecution execution) { return Config.configure(execution); }
}
[Entity({"adapter":"data"})]
[Table("products")]
public sealed class AnnotatedProduct {
    [Id]
    public int id;
}
[CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
public sealed class AnnotatedCatalog {
    [Needs]
    public BehaviorContext behaviors;
    public int calls = 0;
    [Cached]
    public async System.Threading.Tasks.Task<BehaviorJSON> list([PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON page) {
        await System.Threading.Tasks.Task.CompletedTask;
        calls++;
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["calls"] = BehaviorJSON.From(calls), ["offset"] = page.Get("offset") });
    }
    [WebCall({"client":"catalog","method":"GET","path":"/products"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> remote() {
        await System.Threading.Tasks.Task.CompletedTask;
        return BehaviorJSON.Parse("{}");
    }
    [WebCall({"client":"catalog","method":"GET","path":"/products","query":{"page":0}})]
    public async System.Threading.Tasks.Task<BehaviorJSON> search(BehaviorJSON page) {}
    public BehaviorJSON page([PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON input) { return input; }
    [UpdateCache({"method":"list"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> update(BehaviorJSON value, BehaviorJSON page) {
        await System.Threading.Tasks.Task.CompletedTask;
        return value;
    }
    [ClearCache({"method":"list"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> remove(BehaviorJSON page) {
        await System.Threading.Tasks.Task.CompletedTask;
        return BehaviorJSON.From(null);
    }
}
[Store(typeof(AnnotatedProduct), {"fields":["id","name"],"defaultSize":20,"maxSize":100})]
public sealed class AnnotatedQueries {
    [Needs]
    public BehaviorContext behaviors;
    [Query({"operation":"find","where":[{"field":"name","operator":"eq","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]
    public async System.Threading.Tasks.Task<BehaviorJSON> findNamed(BehaviorJSON name, [PageDefaults] BehaviorJSON page) { }
    public async System.Threading.Tasks.Task<BehaviorJSON> countByName(BehaviorJSON name) { }
    public async System.Threading.Tasks.Task<BehaviorJSON> existsByName(BehaviorJSON name) { }
}
public sealed class AnnotatedResilient {
    [Needs]
    public BehaviorContext behaviors;
    public int attempts = 0;
    [Retry({"maxRetries":3,"delayMs":0,"multiplier":1,"maxDelayMs":50,"jitterMs":0})]
    public async System.Threading.Tasks.Task<BehaviorJSON> flaky() {
        await System.Threading.Tasks.Task.CompletedTask;
        attempts++;
        if (attempts < 3) throw new System.InvalidOperationException("transient");
        return BehaviorJSON.From(attempts);
    }
    [MaxAtOnce({"limit":2,"policy":"block"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> bounded(BehaviorJSON value) {
        await System.Threading.Tasks.Task.CompletedTask;
        return value;
    }
}
interface AnnotatedSummaries {
    [Convert({"fields":[{"source":"detail.id","target":"productId"},{"source":"name","target":"title"},{"source":"missing.deep","target":"absent"},{"target":"kind","constant":"product"}]})]
    BehaviorJSON toSummary(BehaviorJSON product);
}
public static class EmittedBehaviorTests {
    private static async System.Threading.Tasks.Task CheckCatalog(BehaviorContext context, BehaviorJSON input) {
        var catalog = new AnnotatedCatalog(context);
        var first = await catalog.list(input);
        var second = await catalog.list(input);
        if (catalog.calls != 1 || first.ToString() != second.ToString() || first.Get("offset").Integer != 6) throw new System.Exception("generated caching and paging failed");
        await context.evict("AnnotatedCatalog", "list", input);
        await catalog.list(input);
        if (catalog.calls != 2 || catalog.page(input).Get("offset").Integer != 6) throw new System.Exception("generated eviction or sync pagination failed");
        var remote = await catalog.remote();
        if (!remote.Get("transport").Boolean) throw new System.Exception("generated HTTP exchange failed");
        var saved = await context.repository(typeof(AnnotatedProduct)).save(BehaviorJSON.Parse("{\"id\":4}"));
        if (saved.Get("table").Text != "products") throw new System.Exception("generated entity mapping failed");
        var replacement = BehaviorJSON.Parse("{\"updated\":true}");
        await catalog.update(replacement, input);
        var updated = await catalog.list(input);
        if (!updated.Get("updated").Boolean || catalog.calls != 2) throw new System.Exception("generated CachePut failed");
        await catalog.remove(input);
        await catalog.list(input);
        if (catalog.calls != 3) throw new System.Exception("generated CacheEvict failed");
    }
    private static async System.Threading.Tasks.Task CheckResilience(BehaviorContext context) {
        var resilient = new AnnotatedResilient(context);
        var attempts = await resilient.flaky();
        if (attempts.Integer != 3 || resilient.attempts != 3) throw new System.Exception("generated Retryable failed");
        var bounded = await resilient.bounded(BehaviorJSON.From("ok"));
        if (bounded.Text != "ok") throw new System.Exception("generated ConcurrencyLimit failed");
    }
    private static async System.Threading.Tasks.Task CheckQueries(BehaviorContext context, BehaviorJSON input) {
        var queries = new AnnotatedQueries(context);
        var found = await queries.findNamed(BehaviorJSON.From("tools"), input);
        if (found.Items.Length != 1) throw new System.Exception("generated Query failed");
        if ((await queries.countByName(BehaviorJSON.From("tools"))).Integer != 1 || !(await queries.existsByName(BehaviorJSON.From("tools"))).Boolean) throw new System.Exception("generated derived queries failed");
    }
    private static void CheckSummaries(BehaviorContext context) {
        var summaries = new AnnotatedSummaries();
        var summary = summaries.toSummary(BehaviorJSON.Parse("{\"detail\":{\"id\":7},\"name\":\"Anvil\",\"extra\":\"dropped\"}"));
        bool mapped = summary.Get("productId").Integer == 7 && summary.Get("title").Text == "Anvil" && summary.Get("absent").IsNull && summary.Get("kind").Text == "product";
        if (!mapped) throw new System.Exception("generated Mapping failed");
        try {
            summaries.toSummary(BehaviorJSON.Parse("[]"));
            throw new System.Exception("a mapping reads one JSON object argument");
        } catch (BehaviorError error) {
            if (error.Code != "JSON_OBJECT") throw;
        }
    }
    public static async System.Threading.Tasks.Task Main() {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(AnnotatedConfig), "tenant-one", signal);
        var input = BehaviorJSON.Parse("{\"page\":2,\"size\":3}");
        await CheckCatalog(context, input);
        await CheckResilience(context);
        await CheckQueries(context, input);
        CheckSummaries(context);
        System.Console.WriteLine("C# emitted annotations: behavior decorators, cache mutations, Query, derived queries and Mapping executed");
    }
}
