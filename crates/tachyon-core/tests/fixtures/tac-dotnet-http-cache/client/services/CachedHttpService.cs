[Service]
[Lifetime({"value":"prototype"})]
public sealed class CachedHttpService
{
    private const string MessageField = "message";
    [Needs]
    private readonly CachedProbeClient _cachedProbeClient;
    public string Last = "ready";
    public int ReadCalls = 0;
    public int SaveCalls = 0;

    public async System.Threading.Tasks.Task<BehaviorJSON> read()
    {
        var answer = await _cachedProbeClient.read();
        Last = answer.Get(MessageField).Text;
        ReadCalls += 1;
        return answer;
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> save()
    {
        var answer = await _cachedProbeClient.save();
        Last = answer.Get(MessageField).Text;
        SaveCalls += 1;
        return answer;
    }
}
