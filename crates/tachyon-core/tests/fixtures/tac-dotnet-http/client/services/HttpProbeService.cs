[Service]
[Lifetime({"value":"prototype"})]
[Description("Ordinary browser HTTP owner and transport conformance")]
public sealed class HttpProbeService
{
    private const string AdapterName = "probe";
    private readonly BehaviorSignal signal = new BehaviorSignal();
    private readonly BehaviorContext context;
    private readonly BehaviorContext customContext;
    public string Last = "ready";
    public string OverrideLast = "ready";

    public HttpProbeService()
    {
        context = Behaviors.create(typeof(BrowserHttpSetup), "http-probe", signal);
        customContext = Behaviors.create(typeof(OverrideHttpSetup), "http-override", signal);
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> Read()
    {
        var answer = await context.restClient(AdapterName).get("/api/probe").retrieve();
        Last = answer.Get("message").Text;
        return answer;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> Slow() => context.restClient(AdapterName).get("/api/slow").retrieve();

    public System.Threading.Tasks.Task<BehaviorJSON> InvalidPath() => context.restClient(AdapterName).get("//untrusted.example").retrieve();

    public async System.Threading.Tasks.Task<BehaviorJSON> Custom()
    {
        var answer = await customContext.restClient(AdapterName).get("/api/probe").retrieve();
        OverrideLast = answer.Get("message").Text;
        return answer;
    }

    [OnStop]
    public void Close()
    {
        context.Dispose();
        customContext.Dispose();
        signal.Dispose();
    }
}
