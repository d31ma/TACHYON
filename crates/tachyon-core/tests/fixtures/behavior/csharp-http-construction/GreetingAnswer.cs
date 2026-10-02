public sealed class GreetingAnswer : BehaviorHTTPAdapter {
    private readonly long value;

    public GreetingAnswer(long value) {
        this.value = value;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
        execution.Check();
        if (request.Get("method").Text != "GET" || request.Get("path").Text != "/api/greeting") {
            throw new System.InvalidOperationException("The actual authored client descriptor was not used.");
        }
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(value));
    }
}

