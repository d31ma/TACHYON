[Service]
[Lifetime({"value":"prototype"})]
[Description("Explicit Setup adapters take precedence over browser transport")]
public sealed class OverrideProbeService
{
    private readonly BehaviorSignal signal = new BehaviorSignal();
    private readonly BehaviorContext context;
    public string Last = "ready";

    public OverrideProbeService()
    {
        context = Behaviors.create(typeof(OverrideHttpSetup), "http-override", signal);
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> Read()
    {
        var answer = await context.restClient("probe").get("/api/probe").retrieve();
        Last = answer.Get("message").Text;
        return answer;
    }

    [OnStop]
    public void Close()
    {
        context.Dispose();
        signal.Dispose();
    }
}
