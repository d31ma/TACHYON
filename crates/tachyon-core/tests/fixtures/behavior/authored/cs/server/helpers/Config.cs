[Description("Behavior conformance Config")]
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
            var answer = new System.Collections.Generic.Dictionary<string, BehaviorJSON> { ["transport"] = BehaviorJSON.From(true), ["path"] = request.Get("path") };
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Object(answer));
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
            if (kind == "count" || kind == "delete") return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(1));
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(true));
        }
    }
}
