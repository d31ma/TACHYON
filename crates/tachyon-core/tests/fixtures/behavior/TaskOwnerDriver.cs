public static class TaskOwnerDriver
{
    private const string First = "6f9619ff-8b86-d011-b42d-00c04fc964ff";
    private const string Second = "7f9619ff-8b86-d011-b42d-00c04fc964ff";
    private static readonly System.Collections.Concurrent.ConcurrentQueue<string> Signals = new();

    public static async System.Threading.Tasks.Task Main()
    {
        Tac.WebEmit = Signals.Enqueue;
        var first = Tac.Invoke(Request(First, "init"));
        var second = Tac.Invoke(Request(Second, "init"));
        Check(first.Contains("\"asyncMethods\":[\"failure\",\"forget\",\"load\"]"), "async members were not advertised");
        Check(second.Contains("\"fields\":[\"count\"]"), "synchronous fields changed");
        Check(Tac.Invoke(Request(First, "set", "count", "\"value\":7")).Contains("\"value\":null"), "set became asynchronous");
        Check(Tac.Invoke(Request(First, "get", "count")).Contains("\"value\":7"), "get became asynchronous");
        var pending = Tac.InvokeAsync(Request(First, "call", "load", "\"args\":[\"first\"]"));
        Check(!pending.IsCompleted, "task was synchronously blocked or replayed");
        Check(Tac.Invoke(Request(First, "get", "count")).Contains("\"value\":8"), "task did not run exactly once");
        Check(Tac.Invoke(Request(Second, "get", "count")).Contains("\"value\":0"), "another owner was blocked or shared");
        TaskSignals.Reentry = Request(Second, "get", "count");
        TaskSignals.Release("first");
        Check((await pending).Contains("\"value\":\"first\""), "Task<BehaviorJSON> did not settle");
        Check(TaskSignals.Reentered.Contains("\"value\":0"), "synchronous reentrant owner call failed");
        Check(Signals.TryDequeue(out var published) && published.Contains(First) && published.Contains("\"retain\":true"), "continuation lost its exact retained-event owner");
        Check(Signals.IsEmpty, "continuation published to more than one owner");

        var forget = Tac.InvokeAsync(Request(Second, "call", "forget"));
        Check(!forget.IsCompleted, "void Task was blocked");
        TaskSignals.Release("void");
        Check((await forget) == "{\"value\":null}", "void Task did not settle to null");
        var failure = await Tac.InvokeAsync(Request(Second, "call", "failure"));
        Check(failure.Contains("error") && !failure.Contains("private failure detail"), "Task failure leaked its exception");
        Check((await Tac.InvokeAsync(Request(Second, "get", "count"))).Contains("error"), "async channel accepted sync operations");
        Check((await Tac.InvokeAsync(Request(Second, "call", "missing"))).Contains("error"), "unknown Task member was accepted");
        Check((await Tac.InvokeAsync(Request(Second, "call", "load", "\"args\":[\"spoofed\"]").Replace("\"web\"", "\"other\""))).Contains("error"), "owner route was spoofed");
        Check((await Tac.InvokeAsync(new string('x', 65537))).Contains("error"), "oversize request was accepted");

        var obsolete = Tac.InvokeAsync(Request(First, "call", "load", "\"args\":[\"obsolete\"]"));
        Tac.Invoke(Request(First, "dispose"));
        Check((await obsolete).Contains("\"code\":\"CANCELLED\""), "dispose did not cancel the waiting result");
        Tac.Invoke(Request(First, "init"));
        TaskSignals.Release("obsolete");
        await TaskSignals.Completed("obsolete").WaitAsync(TimeSpan.FromSeconds(10));
        Check(Signals.IsEmpty, "old continuation published into a reused owner identity");
        Check(Tac.Invoke(Request(First, "get", "count")).Contains("\"value\":0"), "old continuation changed the replacement activation");

        Tac.WebEmit = payload => {
            Signals.Enqueue(payload);
            if (payload.Contains("dispose-during-delivery")) Tac.Invoke(Request(First, "dispose"));
        };
        var reentrantDispose = Tac.InvokeAsync(Request(First, "call", "load", "\"args\":[\"dispose-during-delivery\"]"));
        TaskSignals.Release("dispose-during-delivery");
        Check((await reentrantDispose).Contains("CANCELLED"), "reentrant delivery applied a stale successful result");
        Check(Signals.TryDequeue(out _) && Signals.IsEmpty, "reentrant delivery did not happen exactly once");
        Tac.WebEmit = Signals.Enqueue;
        Check((await Tac.InvokeAsync(Request("8f9619ff-8b86-d011-b42d-00c04fc964ff", "call", "load", "\"args\":[\"unknown-owner\"]"))).Contains("error"), "unknown activation identity was accepted");

        var capped = new List<System.Threading.Tasks.Task<string>>();
        for (var index = 0; index < 64; index++)
            capped.Add(Tac.InvokeAsync(Request(Second, "call", "load", "\"args\":[\"cap-" + index + "\"]")));
        Check((await Tac.InvokeAsync(Request(Second, "call", "load", "\"args\":[\"overflow\"]"))).Contains("error"), "pending Task capacity was unbounded");
        Check(Tac.Invoke(Request(Second, "get", "count")).Contains("\"value\":65"), "capacity rejection invoked or replayed service work");
        Tac.Invoke(Request(Second, "dispose"));
        foreach (var task in capped) Check((await task).Contains("CANCELLED"), "pending Task survived owner disposal");
        for (var index = 0; index < 64; index++) TaskSignals.Release("cap-" + index);
        for (var index = 0; index < 64; index++) await TaskSignals.Completed("cap-" + index).WaitAsync(TimeSpan.FromSeconds(10));
        Check(Signals.IsEmpty, "disposed capacity batch delivered stale events");
        Tac.Invoke(Request(First, "dispose"));
        Console.WriteLine("PASS browser Task ownership and continuation");
    }

    private static string Request(string owner, string operation, string name = null, string extra = null)
    {
        var member = name == null ? "" : ",\"name\":\"" + name + "\"";
        var arguments = extra == null ? "" : "," + extra;
        return "{\"route\":\"web\",\"service\":\"delay\",\"owner\":\"" + owner + "\",\"op\":\"" + operation + "\"" + member + arguments + "}";
    }

    private static void Check(bool accepted, string message)
    {
        if (!accepted) throw new Exception(message);
    }
}
