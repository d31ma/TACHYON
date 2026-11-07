[Delegate]
public sealed class CustomHttpDelegate : BehaviorHTTPAdapter
{
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        RequireRequest(request);
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"message\":\"custom adapter\"}"));
    }
    private static void RequireRequest(BehaviorJSON request)
    {
        switch (request.Get("method").Text, request.Get("path").Text)
        {
            case ("GET", "/api/probe"): return;
            default: throw new System.InvalidOperationException("Unexpected custom adapter request.");
        }
    }
}
