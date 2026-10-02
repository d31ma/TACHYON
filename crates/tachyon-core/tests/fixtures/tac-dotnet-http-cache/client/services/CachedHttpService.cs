[Service]
[Lifetime({"value":"prototype"})]
public sealed class CachedHttpService
{
    [Needs]
    private readonly CachedProbeClient cachedProbeClient;
    public string Last = "ready";
    public int ReadCalls = 0;
    public int SaveCalls = 0;

    public async System.Threading.Tasks.Task<BehaviorJSON> read()
    {
        var answer = await cachedProbeClient.read();
        Last = answer.Get("message").Text;
        ReadCalls += 1;
        return answer;
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> save()
    {
        var answer = await cachedProbeClient.save();
        Last = answer.Get("message").Text;
        SaveCalls += 1;
        return answer;
    }
}
