public partial class Tac
{
    private const int TaskSignalLimit = 65536;
    private const string TaskSignalTooLarge = "{\"error\":\"The task result is larger than 64 KiB.\",\"code\":\"HTTP_LIMIT\"}";
    private const string TaskFailed = "{\"error\":\"Web service task failed.\"}";
    private static long NativeTaskCount;

    internal static string NativeInvoke(string request)
    {
        var parsed = NativeTaskRequest(request);
        if (parsed == null) return Invoke(request);
        return StartNativeTask(request, parsed);
    }

    private static JsonObject NativeTaskRequest(string request)
    {
        try
        {
            if (Encoding.UTF8.GetByteCount(request) > MemberRequestLimit) return null;
            var parsed = JsonNode.Parse(request) as JsonObject;
            lock (OwnerLock) { return NamesLiveTaskMember(parsed) ? parsed : null; }
        }
        catch (Exception) { return null; }
    }

    private static bool NamesLiveTaskMember(JsonObject request)
    {
        if (request == null || request["op"]?.GetValue<string>() != "call") return false;
        var owner = request["owner"]?.GetValue<string>() ?? "";
        if (!Activations.TryGetValue(owner, out var active) || active.Disposed) return false;
        var sameRoute = active.Route == request["route"]?.GetValue<string>();
        var sameService = active.Service == request["service"]?.GetValue<string>();
        var name = request["name"]?.GetValue<string>() ?? "";
        var found = active.Members.TryGetValue(name, out var member);
        return sameRoute && sameService && found && member.InvokeAsync != null;
    }

    private static string StartNativeTask(string request, JsonObject parsed)
    {
        var task = System.Threading.Interlocked.Increment(ref NativeTaskCount);
        var signal = "{\"route\":" + Encode(parsed["route"].GetValue<string>())
            + ",\"service\":" + Encode(parsed["service"].GetValue<string>())
            + ",\"owner\":" + Encode(parsed["owner"].GetValue<string>())
            + ",\"task\":" + task.ToString(CultureInfo.InvariantCulture) + ",\"answer\":";
        var answer = TacOwnerContext.Run(OwnerLock, () => InvokeAsync(request));
        answer.ContinueWith(finished => SendTaskSignal(signal, FinishedAnswer(finished)), System.Threading.Tasks.TaskScheduler.Default);
        return "{\"task\":" + task.ToString(CultureInfo.InvariantCulture) + "}";
    }

    private static string FinishedAnswer(System.Threading.Tasks.Task<string> finished) =>
        finished.IsCompletedSuccessfully ? finished.Result : TaskFailed;

    private static void SendTaskSignal(string signal, string answer)
    {
        var text = signal + Encode(answer) + "}";
        if (Encoding.UTF8.GetByteCount(text) > TaskSignalLimit) text = signal + Encode(TaskSignalTooLarge) + "}";
        SendNativeSignal(text);
    }

    private static unsafe void SendNativeSignal(string text)
    {
        if (Emit == null) return;
        var payload = Marshal.StringToCoTaskMemUTF8(text);
        try { Emit(payload); }
        finally { Marshal.FreeCoTaskMem(payload); }
    }
}
