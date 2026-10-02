public partial class Tac
{
    private const int PendingTaskLimit = 64;
    private const int TaskResultLimit = 1048576;

    public static System.Threading.Tasks.Task<string> InvokeAsync(string request) =>
        TacTaskAnswer.Run(() => CreateTask(request).Run());

    private static TacTaskInvocation CreateTask(string request)
    {
        lock (OwnerLock)
        {
            var parsed = ReadTaskRequest(request);
            var owner = FindTaskOwner(parsed);
            var member = FindTaskMember(owner, parsed);
            var cancellation = owner.Cancellation.Token;
            var work = StartTask(owner, member, TaskArguments(parsed));
            return new TacTaskInvocation(work, cancellation, value => FinishTask(owner, value), () => ReleaseTask(owner));
        }
    }

    private static JsonObject ReadTaskRequest(string request)
    {
        if (request == null || Encoding.UTF8.GetByteCount(request) > MemberRequestLimit)
            throw new ArgumentException("Invalid task request.");
        return JsonNode.Parse(request).AsObject();
    }

    private static Activation FindTaskOwner(JsonObject request)
    {
        var owner = request["owner"]?.GetValue<string>();
        var route = request["route"]?.GetValue<string>();
        var service = request["service"]?.GetValue<string>();
        if (Handling || request["op"]?.GetValue<string>() != "call" || !Guid.TryParseExact(owner, "D", out _)
            || !Activations.TryGetValue(owner, out var active) || active.Route != route || active.Service != service
            || active.Disposed || active.PendingTasks >= PendingTaskLimit)
            throw new ArgumentException("Invalid task owner.");
        return active;
    }

    private static TacMember FindTaskMember(Activation owner, JsonObject request)
    {
        var name = request["name"]?.GetValue<string>();
        if (name == null || !owner.Members.TryGetValue(name, out var member) || member.InvokeAsync == null)
            throw new ArgumentException("Unknown task member.");
        return member;
    }

    private static object[] TaskArguments(JsonObject request)
    {
        if (request["args"] is not JsonArray arguments) return Array.Empty<object>();
        return arguments.Select(Value).ToArray();
    }

    private static System.Threading.Tasks.Task<object> StartTask(Activation owner, TacMember member, object[] arguments)
    {
        var previous = TaskOwner.Value;
        TaskOwner.Value = owner;
        owner.PendingTasks++;
        try { return member.InvokeAsync(arguments); }
        catch (Exception) { owner.PendingTasks--; throw; }
        finally { TaskOwner.Value = previous; }
    }

    private static string FinishTask(Activation owner, object value)
    {
        lock (OwnerLock)
        {
            RequireTaskOwner(owner);
            var json = Encode(value);
            RequireTaskResult(json);
            Drain(owner);
            RequireTaskOwner(owner);
            return "{\"value\":" + json + "}";
        }
    }

    private static void RequireTaskOwner(Activation owner)
    {
        if (owner.Disposed || !Activations.TryGetValue(owner.Owner, out var active) || active != owner)
            throw new System.OperationCanceledException("Disposed task owner.");
    }

    private static void RequireTaskResult(string json)
    {
        if (Encoding.UTF8.GetByteCount(json) > TaskResultLimit) throw new ArgumentException("Task result is too large.");
    }

    private static void ReleaseTask(Activation owner)
    {
        lock (OwnerLock) { owner.PendingTasks--; }
    }
}
