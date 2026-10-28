[Delegate]
sealed class CatalogEchoDelegate : BehaviorHTTPAdapter {
    private static readonly string[] Keys = { "method", "path", "query", "headers", "body" };

    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
        var sent = new System.Collections.Generic.Dictionary<string, BehaviorJSON>();
        foreach (var key in Keys) sent[key] = request.Has(key) ? request.Get(key) : BehaviorJSON.Parse("null");
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Object(sent));
    }
}
