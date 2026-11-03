using System;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;

public static class NativeTaskDriver
{
    private const string Owner = "00000000-0000-4000-8000-000000000001";
    private const string Route = "/notes";
    private const string Notes = "notes";
    private const string Archive = "archive";
    private static readonly System.Collections.Concurrent.BlockingCollection<string> Signals = new();

    public static async System.Threading.Tasks.Task Main()
    {
        Connect();
        await CheckTransport();
        CheckSecondService();
        CheckTasks();
        Console.WriteLine("PASS native C# tasks reach Yon");
    }

    private static unsafe void Connect() =>
        ((delegate* unmanaged<IntPtr, void>)&TacNative.NativeSetEmit)((IntPtr)(delegate* unmanaged<IntPtr, void>)&Receive);

    [UnmanagedCallersOnly]
    private static void Receive(IntPtr payload) => Signals.Add(Marshal.PtrToStringUTF8(payload));

    private static void CheckSecondService()
    {
        var described = Ask(Request(Archive, "init", ""));
        Check(described.Contains("\"asyncMethods\":[\"load\"]"), "the second service was not described: " + described);
        var task = StartTask(Archive, "load");
        var loaded = NextAnswer(Archive, task);
        Check(loaded["value"]?["title"]?.GetValue<string>() == "first", "the second service did not reach Yon: " + loaded.ToJsonString());
        Check(Ask(Request(Archive, "get", ",\"name\":\"count\"")) == "{\"value\":1}", "the second service lost its own state");
    }

    private static void CheckTasks()
    {
        var described = Ask(Request(Notes, "init", ""));
        Check(described.Contains("\"asyncMethods\":[\"large\",\"load\",\"missing\",\"slow\"]"), "async members were not described: " + described);
        Check(Ask(Request(Notes, "call", ",\"name\":\"total\",\"args\":[]")) == "{\"value\":0}", "a synchronous method changed its answer");
        Check(Ask(Request(Notes, "get", ",\"name\":\"count\"")) == "{\"value\":0}", "a field read changed its answer");

        var first = StartTask(Notes, "load");
        var loaded = NextAnswer(Notes, first);
        Check(loaded["value"]?["title"]?.GetValue<string>() == "first", "the task did not answer the Yon value: " + loaded.ToJsonString());
        Check(Ask(Request(Notes, "call", ",\"name\":\"total\",\"args\":[]")) == "{\"value\":2}", "the continuation after await did not run once");

        var second = StartTask(Notes, "missing");
        Check(second > first, "task numbers must increase");
        Check(NextAnswer(Notes, second)["code"]?.GetValue<string>() == "HTTP_STATUS", "a 404 did not answer HTTP_STATUS");

        var large = StartTask(Notes, "large");
        var tooLarge = NextAnswer(Notes, large);
        Check(tooLarge["code"]?.GetValue<string>() == "HTTP_LIMIT", "a signal over 64 KiB was not replaced: " + tooLarge.ToJsonString());

        var slow = StartTask(Notes, "slow");
        Check(Ask(Request(Notes, "dispose", "")) == "{\"value\":null}", "dispose failed");
        Check(NextAnswer(Notes, slow)["code"]?.GetValue<string>() == "CANCELLED", "a disposed owner's task did not finish as CANCELLED");
        Check(Ask(Request(Notes, "call", ",\"name\":\"load\",\"args\":[]")).Contains("error"), "a disposed owner accepted a task");
        Ask("{\"op\":\"host-dispose\"}");
    }

    private static async System.Threading.Tasks.Task CheckTransport()
    {
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/first\""), "{\"value\":{\"title\":\"first\"}}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/missing\""), "{\"error\":{\"code\":\"HTTP_STATUS\",\"status\":404}}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/moved\""), "{\"error\":{\"code\":\"HTTP_NETWORK\"}}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/empty\""), "{\"value\":null}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/broken\""), "{\"error\":{\"code\":\"HTTP_RESPONSE\"}}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/unsafe\""), "{\"error\":{\"code\":\"HTTP_RESPONSE\"}}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/large\"", 1000), "{\"error\":{\"code\":\"HTTP_LIMIT\"}}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/slow\"", 1048576, 300), "{\"error\":{\"code\":\"DEADLINE\"}}");
        await Expect(Http("\"method\":\"GET\",\"path\":\"/api/notes/first\"", 1048576, -1000), "{\"error\":{\"code\":\"DEADLINE\"}}");
        var echoed = await Answer(Http("\"method\":\"POST\",\"path\":\"/api/notes/echo\",\"query\":{\"term\":\"a b&c\",\"page\":2},\"headers\":{\"x-note\":\"kept\"},\"body\":{\"title\":\"sent\"}"));
        var echo = JsonNode.Parse(echoed)["value"];
        Check(echo?["target"]?.GetValue<string>() == "/api/notes/echo?term=a+b%26c&page=2", "query encoding changed: " + echoed);
        Check(echo?["type"]?.GetValue<string>() == "application/json", "a body was sent without application/json: " + echoed);
        Check(echo?["note"]?.GetValue<string>() == "kept" && echo?["cookie"]?.GetValue<string>() == "", "headers changed: " + echoed);
        Check(echo?["body"]?.GetValue<string>() == "{\"title\":\"sent\"}", "the body changed: " + echoed);
        Check(TacNativeHttp.Start("{\"method\":\"GET\"}") == "{\"error\":{\"code\":\"HTTP_REQUEST\"}}", "an incomplete request was accepted");
        Check(TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"/api/notes/first\",\"extra\":1")) == "{\"error\":{\"code\":\"HTTP_REQUEST\"}}", "an unknown key was accepted");
        Check(TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"//other.example/first\"")) == "{\"error\":{\"code\":\"HTTP_ORIGIN\"}}", "another origin was accepted");
        Check(TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"/api/../first\"")) == "{\"error\":{\"code\":\"HTTP_ORIGIN\"}}", "a dot segment was accepted");
        Check(TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"/api/notes/first\"", 0)) == "{\"error\":{\"code\":\"HTTP_LIMIT\"}}", "a zero response limit was accepted");
        Check(TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"/api/notes/first\",\"headers\":{\"Cookie\":\"a=b\"}")) == "{\"error\":{\"code\":\"HTTP_HEADER\"}}", "a cookie header was accepted");
        Check(TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"/api/notes/first\",\"body\":{}")) == "{\"error\":{\"code\":\"HTTP_BODY\"}}", "a GET body was accepted");
        Check(TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"/api/notes/first\",\"query\":{\"big\":9007199254740993}")) == "{\"error\":{\"code\":\"HTTP_RESPONSE\"}}", "an unsafe query number was accepted");
        var cancelled = TacNativeHttp.Start(Http("\"method\":\"GET\",\"path\":\"/api/notes/slow\""));
        var waiting = TacNativeHttp.Answer(cancelled);
        TacNativeHttp.Cancel(cancelled);
        Check(await waiting == "{\"error\":{\"code\":\"CANCELLED\"}}", "cancel did not answer CANCELLED");
    }

    private static long StartTask(string service, string name)
    {
        var answer = Ask(Request(service, "call", ",\"name\":\"" + name + "\",\"args\":[]"));
        var task = JsonNode.Parse(answer)?["task"];
        Check(task != null && task.GetValue<long>() > 0, "a task member did not answer a task number: " + answer);
        return task.GetValue<long>();
    }

    private static JsonNode NextAnswer(string service, long task)
    {
        Check(Signals.TryTake(out var text, TimeSpan.FromSeconds(30)), "no done signal arrived for task " + task);
        var signal = JsonNode.Parse(text);
        Check(signal["route"]?.GetValue<string>() == Route && signal["service"]?.GetValue<string>() == service, "the done signal lost its route: " + text);
        Check(signal["owner"]?.GetValue<string>() == Owner && signal["task"]?.GetValue<long>() == task, "the done signal lost its task: " + text);
        Check(Signals.Count == 0, "a task sent more than one done signal");
        return JsonNode.Parse(signal["answer"].GetValue<string>());
    }

    private static string Request(string service, string operation, string rest) =>
        "{\"route\":\"" + Route + "\",\"service\":\"" + service + "\",\"owner\":\"" + Owner + "\",\"op\":\"" + operation + "\"" + rest + "}";

    private static string Http(string members, int limit = 1048576, int wait = 10000)
    {
        var deadline = DateTimeOffset.UtcNow.ToUnixTimeMilliseconds() + wait;
        return "{" + members + ",\"redirect\":\"error\",\"credentials\":\"omit\",\"deadline\":" + deadline + ",\"maxResponseBytes\":" + limit + "}";
    }

    private static async System.Threading.Tasks.Task<string> Answer(string request)
    {
        var ticket = TacNativeHttp.Start(request);
        if (!Guid.TryParseExact(ticket, "D", out _)) return ticket;
        try { return await TacNativeHttp.Answer(ticket); }
        finally { TacNativeHttp.Cancel(ticket); }
    }

    private static async System.Threading.Tasks.Task Expect(string request, string expected)
    {
        var answer = await Answer(request);
        Check(answer == expected, "wanted " + expected + " but the transport answered " + answer);
    }

    private static unsafe string Ask(string request)
    {
        var text = Marshal.StringToCoTaskMemUTF8(request);
        try
        {
            var answer = ((delegate* unmanaged<IntPtr, IntPtr>)&TacNative.NativeInvoke)(text);
            try { return Marshal.PtrToStringUTF8(answer); }
            finally { ((delegate* unmanaged<IntPtr, void>)&TacNative.NativeFree)(answer); }
        }
        finally { Marshal.FreeCoTaskMem(text); }
    }

    private static void Check(bool passed, string message)
    {
        if (!passed) throw new Exception(message);
    }
}
