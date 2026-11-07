internal static class BehaviorBrowserHttp
{
    internal static System.Func<System.Action, (System.Func<string, System.Threading.CancellationToken, System.Threading.Tasks.Task<string>> Sender, System.IDisposable Watch)> Capture = null;

    internal static BehaviorHTTPAdapter Adapter(System.Action dispose) => Wrap(Sender(dispose));

    private static (System.Func<string, System.Threading.CancellationToken, System.Threading.Tasks.Task<string>> Sender, System.IDisposable Watch) Sender(System.Action dispose) =>
        Behaviors.Environment == "tac" && Capture != null ? Capture(dispose) : default;

    private static BehaviorHTTPAdapter Wrap((System.Func<string, System.Threading.CancellationToken, System.Threading.Tasks.Task<string>> Sender, System.IDisposable Watch) scope) =>
        scope.Sender == null ? null : new BehaviorBrowserHttpAdapter(scope.Sender, scope.Watch);
}
