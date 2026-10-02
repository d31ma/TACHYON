[Helper]
public sealed class GreetingFakeHelper : BehaviorHTTPAdapter
{
    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.Parse("{\"message\":\"Hi\"}"));
    }
}
