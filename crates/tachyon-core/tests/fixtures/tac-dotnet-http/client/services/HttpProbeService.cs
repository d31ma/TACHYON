[Service]
[Lifetime({"value":"prototype"})]
[Description("Ordinary browser HTTP owner and transport conformance")]
public sealed class HttpProbeService
{
    private readonly BehaviorSignal signal = new BehaviorSignal();
    private readonly BehaviorContext context;
    public string Last = "ready";

    public HttpProbeService()
    {
        context = Behaviors.create(typeof(BrowserHttpSetup), "http-probe", signal);
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> Read()
    {
        var answer = await context.restClient("probe").get("/api/probe").retrieve();
        Last = answer.Get("message").Text;
        return answer;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> Slow() => context.restClient("probe").get("/api/slow").retrieve();

    public System.Threading.Tasks.Task<BehaviorJSON> InvalidPath() => context.restClient("probe").get("//untrusted.example").retrieve();

    [OnStop]
    public void Close()
    {
        context.Dispose();
        signal.Dispose();
    }
}
