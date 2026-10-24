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

    static partial void DeliverTask(Activation owner, TacMember listener, string json, bool republish)
    {
        if (owner == null || owner.PendingTasks >= PendingTaskLimit) { EventFailure(); return; }
        object value;
        try { value = Value(JsonNode.Parse(json)); }
        catch (Exception) { EventFailure(); return; }
        owner.PendingTasks++;
        var previous = owner.ListenerTail;
        owner.ListenerTail = TacOwnerContext.Run(OwnerLock, () => RunListenerTask(owner, listener, (value, republish), previous));
    }

    private static System.Threading.Tasks.Task ListenerWork(TacMember listener, object value, bool republish)
    {
        if (listener.ListenAsync != null) return listener.ListenAsync(value);
        var result = listener.Listen(value);
        if (republish) Republish(result, listener.Returns);
        return System.Threading.Tasks.Task.CompletedTask;
    }

    private static async System.Threading.Tasks.Task RunListenerTask(Activation owner, TacMember listener, (object Value, bool Republish) delivery, System.Threading.Tasks.Task previous)
    {
        try
        {
            await previous;
            RequireTaskOwner(owner);
            TaskOwner.Value = owner;
            await ListenerWork(listener, delivery.Value, delivery.Republish).WaitAsync(owner.Cancellation.Token);
            RequireTaskOwner(owner);
            if (!Handling) Drain(owner);
        }
        catch (System.OperationCanceledException) {}
        catch (Exception) { EventFailure(); }
        finally { owner.PendingTasks--; }
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
