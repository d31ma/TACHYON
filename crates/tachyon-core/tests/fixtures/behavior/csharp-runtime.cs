public sealed class Product { }
public sealed class ApplicationProperties {
    public ApplicationProperties() {
        throw new System.Exception("property binding must not instantiate DTOs");
    }
}
public sealed class InvalidProperties { }
public sealed class GuardedConfig {
    public static int Calls;
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        Calls++;
        return Config.configure(execution);
    }
}
public sealed class MissingConfig {
    public static int Calls;
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        Calls++;
        return Config.configure(execution);
    }
}
public sealed class Config {
    public static int Authorizations;
    public static bool Allowed = true;
    public static bool IncludeCache = true;
    public static string LastOperation;
    public static string LastMethod;
    public static BehaviorJSON LastArguments;
    public static bool EntityAllowed = true;
    public static bool IncludeEntityAuthorization = true;
    public static int EntityAuthorizations;
    public static int EntityExecutions;
    public static BehaviorJSON QueryResult;
    public static BehaviorJSON LastQuery;
    public static readonly MemoryCache Store = new MemoryCache();
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        if (IncludeCache) adapters.caches["products"] = Store;
        adapters.http["catalog"] = new Transport();
        adapters.entities["data"] = new Database();
        adapters.authorize = (call, current) => {
            Authorizations++;
            LastOperation = call.Get("operation").Text;
            LastMethod = call.Get("method").Text;
            LastArguments = call.Get("keyArguments");
            if (current.Partition != "tenant-one" || !call.Has("keyArguments")) throw new System.Exception("partition not passed");
            return System.Threading.Tasks.Task.FromResult(Allowed);
        };
        if (IncludeEntityAuthorization) {
            adapters.authorizeEntity = (command, current) => {
                EntityAuthorizations++;
                current.Check();
                if (command.Get("operation").Text != "query") throw new System.Exception("query authorization command");
                return System.Threading.Tasks.Task.FromResult(EntityAllowed);
            };
        }
        return adapters;
    }
}
public sealed class MemoryCache : BehaviorCacheAdapter {
    public int Reads;
    public int Writes;
    public int Deletes;
    public readonly System.Collections.Generic.Dictionary<string, BehaviorJSON> Values = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
    public System.Threading.Tasks.Task<BehaviorJSON> get(string key, BehaviorExecution execution) {
        Reads++;
        if (Values.TryGetValue(key, out var value)) return System.Threading.Tasks.Task.FromResult(value);
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"hit\":false}"));
    }
    public System.Threading.Tasks.Task set(string key, BehaviorJSON value, BehaviorJSON policy, long expiresAt, BehaviorExecution execution) {
        Writes++;
        if (Values.Count >= policy.Get("maxEntries").Integer && !Values.ContainsKey(key)) throw new System.Exception("quota");
        var entry = new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["hit"] = BehaviorJSON.From(true), ["value"] = value, ["expiresAt"] = BehaviorJSON.From(expiresAt) };
        Values[key] = BehaviorJSON.Object(entry);
        return System.Threading.Tasks.Task.CompletedTask;
    }
    public System.Threading.Tasks.Task delete(string key, BehaviorExecution execution) {
        Deletes++;
        Values.Remove(key);
        return System.Threading.Tasks.Task.CompletedTask;
    }
}
public sealed class Transport : BehaviorHTTPAdapter {
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
        if (request.Get("redirect").Text != "error" || request.Get("credentials").Text != "omit" || request.Get("maxResponseBytes").Integer != 1048576) throw new System.Exception("transport bounds missing");
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"transport\":true}"));
    }
}
public sealed class Database : BehaviorEntityAdapter {
    public System.Threading.Tasks.Task<BehaviorJSON> execute(BehaviorJSON command, BehaviorExecution execution) {
        Config.EntityExecutions++;
        if (command.Get("table").Text != "products" || command.Get("idField").Text != "id") throw new System.Exception("mapping missing");
        if (command.Get("operation").Text != "query") return System.Threading.Tasks.Task.FromResult(command);
        Config.LastQuery = command;
        if (Config.QueryResult != null) return System.Threading.Tasks.Task.FromResult(Config.QueryResult);
        string kind = command.Get("query").Get("kind").Text;
        if (kind == "find") return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("[{\"id\":1}]"));
        if (kind == "count") return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(1));
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(true));
    }
}
public static class NativeBehaviorTests {
    private static int loadCalls;
    private static int mutationCalls;
    private static void Assert(bool condition, string message) {
        if (!condition) throw new System.Exception(message);
    }
    private static void ExpectError(string code, System.Action action) {
        try {
            action();
        } catch (BehaviorError error) {
            Assert(error.Code == code, "wrong code " + error.Code);
            return;
        }
        throw new System.Exception("Expected " + code);
    }
    private static async System.Threading.Tasks.Task ExpectErrorAsync(string code, System.Func<System.Threading.Tasks.Task> action) {
        try {
            await action();
        } catch (BehaviorError error) {
            Assert(error.Code == code, "wrong code " + error.Code);
            return;
        }
        throw new System.Exception("Expected " + code);
    }
    private static void ExpectRejectedProperties(System.Type type, System.Collections.Generic.Dictionary<string, string> supplied) {
        ExpectError("configuration-properties", () => Behaviors.properties(type, supplied));
    }
    private static System.Threading.Tasks.Task<BehaviorJSON> Load() {
        loadCalls++;
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"result\":1}"));
    }
    private static System.Threading.Tasks.Task<BehaviorJSON> CountMutation(BehaviorJSON result) {
        mutationCalls++;
        return System.Threading.Tasks.Task.FromResult(result);
    }
    private static BehaviorJSON QueryMetadata(string operation) {
        string entity = "{\"name\":\"Product\",\"adapter\":\"data\",\"id\":\"id\",\"table\":\"products\"}";
        string where = "[{\"field\":\"name\",\"operator\":\"eq\",\"argument\":0},{\"field\":\"id\",\"operator\":\"in\",\"argument\":1},{\"field\":\"name\",\"operator\":\"isNotNull\"}]";
        string pageArgument = "null";
        if (operation == "find") pageArgument = "2";
        string orderBy = "[{\"field\":\"id\",\"direction\":\"desc\"}]";
        return BehaviorJSON.Parse("{\"entity\":" + entity + ",\"fields\":[\"id\",\"name\"],\"defaultSize\":20,\"maxSize\":100,\"operation\":\"" + operation + "\",\"where\":" + where + ",\"pageArgument\":" + pageArgument + ",\"orderBy\":" + orderBy + "}");
    }
    private static void CheckPropertyBinding() {
        var supplied = new System.Collections.Generic.Dictionary<string, string> { ["TAC_APP_LABEL"] = "catalog", ["TAC_APP_ENABLED"] = "true", ["TAC_APP_WORKERS"] = "8", ["TAC_APP_RATIO"] = "2.5e0", ["TAC_OTHER_VALUE"] = "ignored" };
        var bound = Behaviors.properties(typeof(ApplicationProperties), supplied);
        Assert(bound.Get("label").Text == "catalog" && bound.Get("enabled").Boolean && bound.Get("workers").Integer == 8 && bound.Get("ratio").ToString() == "2.5", "explicit typed property binding");
        supplied["TAC_APP_WORKERS"] = "16";
        Assert(bound.Get("workers").Integer == 8, "property result snapshot");
        var defaults = Behaviors.properties(typeof(ApplicationProperties), new System.Collections.Generic.Dictionary<string, string> { ["TAC_APP_LABEL"] = "catalog" });
        var nested = Behaviors.properties(typeof(ApplicationProperties), BehaviorJSON.Parse("{\"tac\":{\"app\":{\"label\":\"catalog\",\"workers\":8,\"enabled\":true}}}"));
        Assert(nested.Get("workers").Integer == 8 && nested.Get("enabled").Boolean, "namespaced typed binding");
        Assert(defaults.Get("workers").Integer == 4 && !defaults.Get("enabled").Boolean, "typed property defaults");
        ExpectRejectedProperties(typeof(ApplicationProperties), new System.Collections.Generic.Dictionary<string, string>());
        ExpectRejectedProperties(typeof(InvalidProperties), new System.Collections.Generic.Dictionary<string, string>());
        CheckRejectedPropertyValues(supplied);
    }
    private static void CheckRejectedPropertyValues(System.Collections.Generic.Dictionary<string, string> supplied) {
        foreach (var invalid in new[] { "01", "+1", "1.0", "1e0", " 1", "1\n", "9007199254740992", "0", "65" }) {
            supplied["TAC_APP_WORKERS"] = invalid;
            ExpectRejectedProperties(typeof(ApplicationProperties), supplied);
        }
        supplied["TAC_APP_WORKERS"] = "8";
        supplied["TAC_APP_ENABLED"] = "True";
        ExpectRejectedProperties(typeof(ApplicationProperties), supplied);
        supplied["TAC_APP_ENABLED"] = "true";
        supplied["TAC_APP_TYPO"] = "hidden";
        ExpectRejectedProperties(typeof(ApplicationProperties), supplied);
        supplied.Remove("TAC_APP_TYPO");
        supplied["YON_APP_LABEL"] = "hidden";
        ExpectRejectedProperties(typeof(ApplicationProperties), supplied);
        supplied.Remove("YON_APP_LABEL");
        supplied["TAC_APP_LABEL"] = new string('x', 4097);
        ExpectRejectedProperties(typeof(ApplicationProperties), supplied);
        supplied["TAC_APP_LABEL"] = "catalog";
    }
    private static void CheckConfigurationSelection(BehaviorSignal signal) {
        ExpectError("inactive-configuration", () => Behaviors.create(typeof(GuardedConfig), "tenant-one", signal));
        ExpectError("inactive-configuration", () => Behaviors.create(typeof(GuardedConfig), "tenant-one", signal, profiles: new[] { "testing" }));
        Assert(GuardedConfig.Calls == 0, "inactive configuration never executes factory");
        var selectedProfiles = new[] { "testing" };
        var selectedProperties = new System.Collections.Generic.Dictionary<string, string> { ["TAC_CACHE_ENABLED"] = "true" };
        using var selectedContext = Behaviors.create(typeof(GuardedConfig), "tenant-one", signal, profiles: selectedProfiles, properties: selectedProperties);
        Assert(GuardedConfig.Calls == 1, "combined configuration predicates");
        selectedProfiles[0] = "disabled";
        selectedProperties["TAC_CACHE_ENABLED"] = "false";
        Assert(selectedContext.Paginate(() => 42) == 42 && GuardedConfig.Calls == 1, "creation options are snapshots");
        using var nestedContext = Behaviors.create(typeof(GuardedConfig), "tenant-one", signal, BehaviorJSON.Parse("{\"tac\":{\"cache\":{\"enabled\":true}}}"), profiles: new[] { "testing" });
        using var missingContext = Behaviors.create(typeof(MissingConfig), "tenant-one", signal);
        Assert(MissingConfig.Calls == 1, "matchIfMissing policy");
        ExpectError("configuration", () => Behaviors.create(typeof(Config), "tenant-one", signal, profiles: new[] { "dev", "dev" }));
        ExpectError("configuration", () => Behaviors.create(typeof(Config), "tenant-one", signal, properties: new System.Collections.Generic.Dictionary<string, string> { ["YON_WRONG"] = "hidden" }));
    }
    private static async System.Threading.Tasks.Task CheckCachedCalls(BehaviorContext context, BehaviorJSON argument, BehaviorJSON[] listArguments) {
        await context.Invoke("Catalog", "list", listArguments, Load);
        await context.Invoke("Catalog", "list", listArguments, Load);
        Assert(loadCalls == 1 && Config.Authorizations == 2, "cache hit/miss authorization");
        await context.evict("Catalog", "list", argument);
        await context.Invoke("Catalog", "list", listArguments, Load);
        Assert(loadCalls == 2 && Config.Authorizations == 4, "eviction");
        Config.Allowed = false;
        await ExpectErrorAsync("FORBIDDEN", async () => { await context.Invoke("Catalog", "list", listArguments, Load); });
        Config.Allowed = true;
        var http = await context.Invoke("Catalog", "remote", System.Array.Empty<BehaviorJSON>(), () => System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{}")));
        Assert(http.Get("transport").Boolean, "http exchange");
    }
    private static async System.Threading.Tasks.Task CheckRepository(BehaviorContext context, BehaviorJSON argument) {
        var repository = context.repository(typeof(Product));
        Assert((await repository.find(argument)).Get("page").Get("offset").Integer == 2, "repository page");
        Assert((await repository.save(BehaviorJSON.Parse("{\"id\":4}"))).Get("operation").Text == "save", "repository save");
        Assert((await repository.delete(BehaviorJSON.From(4))).Get("id").Integer == 4, "repository delete");
    }
    private static async System.Threading.Tasks.Task CheckStrictInput(BehaviorContext context, BehaviorJSON pagePolicy) {
        ExpectError("PAGE_SORT", () => BehaviorPage.Normalize(BehaviorJSON.Parse("{\"sort\":[\"unknown\"]}"), pagePolicy));
        ExpectError("JSON_DUPLICATE", () => BehaviorJSON.Parse("{\"x\":1,\"x\":2}"));
        ExpectError("JSON_NUMBER", () => BehaviorJSON.Parse("1e999"));
        await ExpectErrorAsync("HTTP_HEADER", async () => { await context.Invoke("Catalog", "remote", System.Array.Empty<BehaviorJSON>(), () => System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"headers\":{\"x\":\"bad\\r\\nvalue\"}}"))); });
        await ExpectErrorAsync("HTTP_HEADER", async () => { await context.Invoke("Catalog", "remote", System.Array.Empty<BehaviorJSON>(), () => System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"headers\":{\"Cookie\":\"session=invalid\"}}"))); });
        await ExpectErrorAsync("HTTP_BODY", async () => { await context.Invoke("Catalog", "remote", System.Array.Empty<BehaviorJSON>(), () => System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"body\":null}"))); });
        ExpectError("JSON_TYPE", () => BehaviorJSON.From(new object()));
        Assert(BehaviorJSON.Parse("1.0").ToString() == BehaviorJSON.Parse("1").ToString(), "numeric key canonicalization");
        Assert(BehaviorJSON.Parse("\"\\u0041\"").ToString() == BehaviorJSON.Parse("\"A\"").ToString(), "string key canonicalization");
    }
    private static async System.Threading.Tasks.Task CheckOperationLimit(BehaviorContext context) {
        var blocker = new System.Threading.Tasks.TaskCompletionSource<BehaviorJSON>();
        var operations = new System.Collections.Generic.List<System.Threading.Tasks.Task<BehaviorJSON>>();
        for (var index = 0; index < 64; index++) {
            operations.Add(context.Invoke("Catalog", "remote", System.Array.Empty<BehaviorJSON>(), () => blocker.Task));
        }
        await ExpectErrorAsync("OPERATION_LIMIT", async () => { await context.Invoke("Catalog", "remote", System.Array.Empty<BehaviorJSON>(), () => blocker.Task); });
        blocker.SetResult(BehaviorJSON.Parse("{}"));
        await System.Threading.Tasks.Task.WhenAll(operations);
    }
    private static async System.Threading.Tasks.Task CheckCachePut(BehaviorContext context, BehaviorJSON argument, BehaviorJSON[] listArguments, BehaviorJSON replacement) {
        int readsBefore = Config.Store.Reads;
        int writesBefore = Config.Store.Writes;
        await context.Invoke("Catalog", "update", new[] { replacement, argument }, () => CountMutation(replacement));
        Assert(mutationCalls == 1 && Config.Store.Reads == readsBefore && Config.Store.Writes == writesBefore + 1, "CachePut has no read and stores exactly once");
        Assert(Config.LastOperation == "cache:put" && Config.LastMethod == "list" && Config.LastArguments.Items[0].Get("offset").Integer == 2, "mutation authorizes normalized target");
        Assert((await context.Invoke("Catalog", "list", listArguments, Load)).Get("updated").Boolean && loadCalls == 2, "CachePut refreshes target key");
    }
    private static async System.Threading.Tasks.Task CheckCacheEvict(BehaviorContext context, BehaviorJSON argument, BehaviorJSON[] listArguments) {
        int deletesBefore = Config.Store.Deletes;
        await context.Invoke("Catalog", "remove", new[] { argument }, () => CountMutation(BehaviorJSON.From(null)));
        Assert(Config.Store.Deletes == deletesBefore + 1 && Config.LastOperation == "cache:evict", "CacheEvict deletes after successful mutation");
        await context.Invoke("Catalog", "list", listArguments, Load);
        Assert(loadCalls == 3, "CacheEvict invalidated target");
    }
    private static async System.Threading.Tasks.Task CheckRejectedMutations(BehaviorContext context, BehaviorSignal signal, BehaviorJSON argument, BehaviorJSON replacement) {
        int writesBefore = Config.Store.Writes;
        int deletesBefore = Config.Store.Deletes;
        Config.Allowed = false;
        await ExpectErrorAsync("FORBIDDEN", async () => { await context.Invoke("Catalog", "update", new[] { replacement, argument }, () => CountMutation(replacement)); });
        Config.Allowed = true;
        Assert(mutationCalls == 2 && Config.Store.Writes == writesBefore, "denied mutation never executes");
        await ExpectErrorAsync("CACHE_ARGUMENT", async () => { await context.Invoke("Catalog", "update", new[] { replacement }, () => CountMutation(replacement)); });
        Config.IncludeCache = false;
        using var missingAdapter = Behaviors.create(typeof(Config), "tenant-one", signal);
        Config.IncludeCache = true;
        await ExpectErrorAsync("CACHE_MISSING", async () => { await missingAdapter.Invoke("Catalog", "update", new[] { replacement, argument }, () => CountMutation(replacement)); });
        Assert(mutationCalls == 2, "invalid target arguments and missing adapters fail before mutation");
        await ExpectErrorAsync("APPLICATION_FAILED", async () => { await context.Invoke("Catalog", "remove", new[] { argument }, () => { throw new BehaviorError("APPLICATION_FAILED"); }); });
        Assert(Config.Store.Deletes == deletesBefore, "failed mutation never evicts");
        await ExpectErrorAsync("JSON_LIMIT", async () => { await context.Invoke("Catalog", "update", new[] { replacement, argument }, () => System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(new string('x', 65537)))); });
        Assert(Config.Store.Writes == writesBefore, "oversized mutation never stores");
    }
    private static async System.Threading.Tasks.Task CheckHttpCachePut(BehaviorContext context, BehaviorJSON argument, BehaviorJSON[] listArguments) {
        await context.Invoke("Catalog", "updateRemote", new[] { argument }, () => System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{}")));
        Assert((await context.Invoke("Catalog", "list", listArguments, Load)).Get("transport").Boolean, "CachePut stores HttpExchange result");
    }
    private static async System.Threading.Tasks.Task CheckInterruptedMutations(BehaviorSignal signal, BehaviorJSON argument, BehaviorJSON replacement) {
        using var cancelledMutation = Behaviors.create(typeof(Config), "tenant-one", signal);
        int writesBefore = Config.Store.Writes;
        await ExpectErrorAsync("CANCELLED", async () => { await cancelledMutation.Invoke("Catalog", "update", new[] { replacement, argument }, () => { cancelledMutation.dispose(); return System.Threading.Tasks.Task.FromResult(replacement); }); });
        Assert(Config.Store.Writes == writesBefore, "cancelled mutation never stores");
        using var timedMutation = Behaviors.create(typeof(Config), "tenant-one", signal, 10);
        await ExpectErrorAsync("DEADLINE", async () => { await timedMutation.Invoke("Catalog", "update", new[] { replacement, argument }, async () => { await System.Threading.Tasks.Task.Delay(30); return replacement; }); });
        Assert(Config.Store.Writes == writesBefore, "expired mutation never stores");
    }
    private static async System.Threading.Tasks.Task CheckQueries(BehaviorContext context, BehaviorSignal signal) {
        var queryArguments = new[] { BehaviorJSON.From("tools"), BehaviorJSON.Parse("[1,2]"), BehaviorJSON.Parse("{\"page\":1,\"size\":2}") };
        await CheckAuthorizedQueries(context, queryArguments);
        await CheckRejectedQueries(context, signal, queryArguments);
        await CheckInvalidQueryResults(context, queryArguments);
    }
    private static async System.Threading.Tasks.Task CheckAuthorizedQueries(BehaviorContext context, BehaviorJSON[] queryArguments) {
        int executionsBefore = Config.EntityExecutions;
        int authorizationsBefore = Config.EntityAuthorizations;
        var rows = await context.Query(QueryMetadata("find"), queryArguments);
        Assert(rows.Items.Length == 1 && Config.EntityExecutions == executionsBefore + 1 && Config.EntityAuthorizations == authorizationsBefore + 1, "query executes through authorization");
        var query = Config.LastQuery.Get("query");
        Assert(query.Get("page").Get("offset").Integer == 2 && query.Get("orderBy").Items[0].Get("direction").Text == "desc", "query paging and static ordering");
        Assert(query.Get("where").Items[1].Get("value").Items.Length == 2 && !query.Get("where").Items[2].Has("value"), "query bindings are structured data");
        Assert((await context.Query(QueryMetadata("count"), queryArguments)).Integer == 1 && !Config.LastQuery.Get("query").Has("page"), "query count shape");
        Assert((await context.Query(QueryMetadata("exists"), queryArguments)).Boolean, "query exists shape");
    }
    private static async System.Threading.Tasks.Task CheckRejectedQueries(BehaviorContext context, BehaviorSignal signal, BehaviorJSON[] queryArguments) {
        int executionsBefore = Config.EntityExecutions;
        Config.EntityAllowed = false;
        await ExpectErrorAsync("FORBIDDEN", async () => { await context.Query(QueryMetadata("find"), queryArguments); });
        Config.EntityAllowed = true;
        Config.IncludeEntityAuthorization = false;
        using var unauthenticatedQuery = Behaviors.create(typeof(Config), "tenant-one", signal);
        Config.IncludeEntityAuthorization = true;
        await ExpectErrorAsync("FORBIDDEN", async () => { await unauthenticatedQuery.Query(QueryMetadata("find"), queryArguments); });
        Assert(Config.EntityExecutions == executionsBefore, "missing and denied query authorization never executes");
        await ExpectErrorAsync("QUERY_BINDING", async () => { await context.Query(QueryMetadata("find"), new[] { BehaviorJSON.Parse("{}"), queryArguments[1], queryArguments[2] }); });
        var tooManyValues = new BehaviorJSON[101];
        System.Array.Fill(tooManyValues, BehaviorJSON.From(1));
        await ExpectErrorAsync("QUERY_BINDING", async () => { await context.Query(QueryMetadata("find"), new[] { queryArguments[0], BehaviorJSON.Array(tooManyValues), queryArguments[2] }); });
        await ExpectErrorAsync("JSON_LIMIT", async () => { await context.Query(QueryMetadata("find"), new[] { BehaviorJSON.From(new string('x', 65536)), queryArguments[1], queryArguments[2] }); });
        await ExpectErrorAsync("QUERY_ORDER", async () => { await context.Query(QueryMetadata("find"), new[] { queryArguments[0], queryArguments[1], BehaviorJSON.Parse("{\"sort\":[\"id\"]}") }); });
        Assert(Config.EntityExecutions == executionsBefore, "invalid queries fail before adapter execution");
    }
    private static async System.Threading.Tasks.Task CheckInvalidQueryResults(BehaviorContext context, BehaviorJSON[] queryArguments) {
        Config.QueryResult = BehaviorJSON.Parse("[{},{},{}]");
        await ExpectErrorAsync("QUERY_RESULT", async () => { await context.Query(QueryMetadata("find"), queryArguments); });
        Config.QueryResult = BehaviorJSON.Parse("[1]");
        await ExpectErrorAsync("JSON_OBJECT", async () => { await context.Query(QueryMetadata("find"), queryArguments); });
        Config.QueryResult = BehaviorJSON.From(-1);
        await ExpectErrorAsync("QUERY_RESULT", async () => { await context.Query(QueryMetadata("count"), queryArguments); });
        Config.QueryResult = BehaviorJSON.From(1);
        await ExpectErrorAsync("JSON_BOOLEAN", async () => { await context.Query(QueryMetadata("exists"), queryArguments); });
        Config.QueryResult = null;
    }
    private static async System.Threading.Tasks.Task CheckDeadlineAndCancellation(BehaviorContext context, BehaviorSignal signal, BehaviorJSON[] listArguments) {
        using var shortContext = Behaviors.create(typeof(Config), "tenant-one", signal, 10);
        await ExpectErrorAsync("DEADLINE", async () => { await shortContext.Invoke("Catalog", "remote", System.Array.Empty<BehaviorJSON>(), async () => { await System.Threading.Tasks.Task.Delay(30); return BehaviorJSON.Parse("{}"); }); });
        context.dispose();
        await ExpectErrorAsync("CANCELLED", async () => { await context.Invoke("Catalog", "list", listArguments, Load); });
        using var ownerContext = Behaviors.create(typeof(Config), "tenant-one", signal);
        signal.Abort();
        await ExpectErrorAsync("CANCELLED", async () => { await ownerContext.Invoke("Catalog", "list", listArguments, Load); });
    }
    public static async System.Threading.Tasks.Task Main() {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(Config), "tenant-one", signal);
        CheckPropertyBinding();
        CheckConfigurationSelection(signal);
        var argument = BehaviorJSON.Parse("{\"page\":1,\"size\":2,\"sort\":[\"id\"]}");
        var pagePolicy = BehaviorJSON.Parse("{\"defaultSize\":20,\"maxSize\":100,\"sort\":[\"id\"],\"argument\":0}");
        var normalized = BehaviorPage.Normalize(argument, pagePolicy);
        Assert(normalized.Get("offset").Integer == 2 && normalized.Get("sort").Items[0].Text == "id:asc", "pagination");
        var listArguments = new[] { normalized };
        await CheckCachedCalls(context, argument, listArguments);
        await CheckRepository(context, argument);
        await CheckStrictInput(context, pagePolicy);
        await CheckOperationLimit(context);
        var replacement = BehaviorJSON.Parse("{\"updated\":true}");
        await CheckCachePut(context, argument, listArguments, replacement);
        await CheckCacheEvict(context, argument, listArguments);
        await CheckRejectedMutations(context, signal, argument, replacement);
        await CheckHttpCachePut(context, argument, listArguments);
        await CheckInterruptedMutations(signal, argument, replacement);
        await CheckQueries(context, signal);
        await CheckDeadlineAndCancellation(context, signal, listArguments);
        System.Console.WriteLine("C# behavior runtime: cache mutations, authorized queries, pagination, HTTP, persistence, strict JSON, deadlines, disposal passed");
    }
}
