[HTTPClient({"path":"/api/greeting","adapter":"greeting"})]
public sealed class GreetingClient {
    [Needs]
    private readonly BehaviorContext behaviors;

    [GetRequest]
    public async System.Threading.Tasks.Task<BehaviorJSON> read() {}
}

