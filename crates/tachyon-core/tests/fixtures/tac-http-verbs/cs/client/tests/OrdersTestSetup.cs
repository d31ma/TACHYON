[TestSetup]
public sealed class OrdersTestSetup
{
    sealed class EchoTransport : BehaviorHTTPAdapter
    {
        public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
        {
            execution.Check();
            var headers = request.Get("headers");
            var header = headers.Has("X-Tenant") ? headers.Get("X-Tenant") : headers.Get("X-Reason");
            var item = request.Has("body") ? request.Get("body").Get("item") : BehaviorJSON.From("none");
            return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON>
            {
                ["method"] = request.Get("method"),
                ["path"] = request.Get("path"),
                ["header"] = header,
                ["item"] = item,
            }));
        }
    }

    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["orders"] = new EchoTransport();
        return adapters;
    }
}
