[Helper]
public sealed class CustomHttpHelper : BehaviorHTTPAdapter
{
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        if (request.Get("method").Text != "GET" || request.Get("path").Text != "/api/probe") throw new System.InvalidOperationException("Unexpected custom adapter request.");
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"message\":\"custom adapter\"}"));
    }
}
