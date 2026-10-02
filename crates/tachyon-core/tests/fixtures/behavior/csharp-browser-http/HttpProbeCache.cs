internal sealed class HttpProbeCache : BehaviorCacheAdapter
{
    private readonly System.Collections.Generic.Dictionary<string, BehaviorJSON> values = new();

    public System.Threading.Tasks.Task<BehaviorJSON> get(string key, BehaviorExecution execution) =>
        System.Threading.Tasks.Task.FromResult(values.GetValueOrDefault(key, BehaviorJSON.Parse("{\"hit\":false}")));

    public System.Threading.Tasks.Task set(string key, BehaviorJSON value, BehaviorJSON policy, long expiresAt, BehaviorExecution execution)
    {
        values[key] = BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON>
        {
            ["hit"] = BehaviorJSON.From(true), ["value"] = value, ["expiresAt"] = BehaviorJSON.From(expiresAt),
        });
        return System.Threading.Tasks.Task.CompletedTask;
    }

    public System.Threading.Tasks.Task delete(string key, BehaviorExecution execution)
    {
        values.Remove(key);
        return System.Threading.Tasks.Task.CompletedTask;
    }
}
