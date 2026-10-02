internal static class HttpProbeBindings
{
    internal static HttpProbeTransport Current;
    internal static int ConfigurationCalls;
    internal static readonly HttpProbeCache Store = new();
    internal static readonly HttpProbeOverride Override = new();

    internal static (System.Func<string, System.Threading.CancellationToken, System.Threading.Tasks.Task<string>> Sender, System.IDisposable Watch) Capture(System.Action dispose) =>
        (Current.Send, Current.Lifetime.Token.Register(dispose));

    internal static BehaviorAdapters Configuration(bool custom)
    {
        ConfigurationCalls++;
        var adapters = new BehaviorAdapters();
        adapters.caches["probe"] = Store;
        adapters.authorize = (request, execution) => System.Threading.Tasks.Task.FromResult(true);
        if (custom) adapters.http["probe"] = Override;
        return adapters;
    }
}
