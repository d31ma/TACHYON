[Service]
[Lifetime({"value":"prototype"})]
public sealed class CachedHttpService
{
    private const string MessageField = "message";
    private const string ReadyMessage = "ready";
    [Needs]
    private readonly CachedProbeClient _cachedProbeClient;
    [Needs]
    private readonly WorkerProbeClient _workerProbeClient;
    [Needs]
    private readonly OfflineProbeClient _offlineProbeClient;
    public string Last = ReadyMessage;
    public string WorkerLast = ReadyMessage;
    public string OfflineLast = ReadyMessage;
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

    public async System.Threading.Tasks.Task<BehaviorJSON> workerRead()
    {
        var answer = await _workerProbeClient.read();
        WorkerLast = answer.Get(MessageField).Text;
        return answer;
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> slow()
    {
        var answer = await _cachedProbeClient.slow();
        Last = answer.Get(MessageField).Text;
        return answer;
    }

    public async System.Threading.Tasks.Task<BehaviorJSON> offlineRead()
    {
        var answer = await _offlineProbeClient.read();
        OfflineLast = answer.Get(MessageField).Text;
        return answer;
    }
}
