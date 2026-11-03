internal sealed class HttpProbeOverride : BehaviorHTTPAdapter
{
    internal readonly System.Collections.Generic.List<BehaviorJSON> Requests = new();

    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        Requests.Add(request);
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"owner\":\"custom\"}"));
    }
}
