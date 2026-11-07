public partial class Tac
{
    private const string SocketSent = "{\"value\":null}";

    internal static System.Threading.Tasks.Task<string> SendNativeSocket(string owner, string request)
    {
        string signal;
        lock (OwnerLock) { signal = NativeSocketSignal(owner, request); }
        if (signal == null || !NativeEmitReady()) return System.Threading.Tasks.Task.FromResult(TacSocketTransport.Missing);
        SendNativeSignal(signal);
        return System.Threading.Tasks.Task.FromResult(SocketSent);
    }

    private static string NativeSocketSignal(string owner, string request)
    {
        var live = Activations.TryGetValue(owner, out var active) && !active.Disposed;
        var parsed = live ? JsonNode.Parse(request) as JsonObject : null;
        if (parsed == null) return null;
        return "{\"route\":" + Encode(active.Route) + ",\"service\":" + Encode(active.Service) + ",\"owner\":" + Encode(owner)
            + ",\"send\":" + Encode(parsed["client"]?.GetValue<string>()) + ",\"value\":" + (parsed["message"]?.ToJsonString() ?? "null") + "}";
    }

    private static unsafe bool NativeEmitReady() => Emit != null;
}
