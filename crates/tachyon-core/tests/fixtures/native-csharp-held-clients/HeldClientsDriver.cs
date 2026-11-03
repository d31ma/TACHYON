using System;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;

public static class HeldClientsDriver
{
    private const string ChatOwner = "00000000-0000-4000-8000-000000000001";
    private const string StreamOwner = "00000000-0000-4000-8000-000000000002";
    private const string Route = "/";
    private static readonly System.Collections.Concurrent.BlockingCollection<string> Signals = new();

    public static void Main()
    {
        Connect();
        CheckSocket();
        CheckStream();
        Ask("{\"op\":\"host-dispose\"}");
        Console.WriteLine("PASS linked C# services hand their clients to the page");
    }

    private static unsafe void Connect() =>
        ((delegate* unmanaged<IntPtr, void>)&TacNative.NativeSetEmit)((IntPtr)(delegate* unmanaged<IntPtr, void>)&Receive);

    [UnmanagedCallersOnly]
    private static void Receive(IntPtr payload) => Signals.Add(Marshal.PtrToStringUTF8(payload));

    private static void CheckSocket()
    {
        var described = JsonNode.Parse(Ask(Request(ChatOwner, "chat", "init", "")))["value"];
        var stream = Only(described["streams"], "the chat service");
        Check(stream["client"]?.GetValue<string>() == "ChatClient" && stream["path"]?.GetValue<string>() == "/chat", "the socket client was not described: " + described.ToJsonString());
        Check(stream["socket"]?.GetValue<bool>() == true && stream["event"]?.GetValue<string>() == "ChatMessage", "the socket kind was lost: " + described.ToJsonString());
        var listener = Only(described["listeners"], "the chat service");
        Check(listener["events"]?[0]?.GetValue<string>() == "ChatMessage", "the page cannot deliver the socket's messages: " + described.ToJsonString());
        var key = listener["key"].GetValue<string>();
        var heard = Ask(Request(ChatOwner, "chat", "event", ",\"name\":\"" + key + "\",\"value\":{\"text\":\"from the page\"}"));
        Check(heard == "{\"value\":null}", "the listener refused a page message: " + heard);
        var messages = Ask(Request(ChatOwner, "chat", "get", ",\"name\":\"" + Only(described["fields"], "the chat service").GetValue<string>() + "\""));
        Check(messages.Contains("from the page"), "the listener did not keep the message: " + messages);
        Check(Signals.Count == 0, "hearing a message sent a signal");

        var sayHello = Only(described["methods"], "the chat service").GetValue<string>();
        var started = JsonNode.Parse(Ask(Request(ChatOwner, "chat", "call", ",\"name\":\"" + sayHello + "\",\"args\":[]")));
        Check(Signals.TryTake(out var sentText, TimeSpan.FromSeconds(30)), "no socket signal arrived after " + started.ToJsonString());
        var sent = JsonNode.Parse(sentText);
        Check(sent["send"]?.GetValue<string>() == "ChatClient", "the socket message did not go to the page: " + sent.ToJsonString());
        Check(sent["route"]?.GetValue<string>() == Route && sent["service"]?.GetValue<string>() == "chat" && sent["owner"]?.GetValue<string>() == ChatOwner, "the socket message lost its owner: " + sent.ToJsonString());
        Check(sent["value"]?["text"]?.GetValue<string>() == "Hello from a tab", "the socket message changed: " + sent.ToJsonString());
        var task = started["task"];
        if (task != null)
        {
            var done = Next();
            Check(done["task"]?.GetValue<long>() == task.GetValue<long>(), "the task did not finish: " + done.ToJsonString());
            Check(done["answer"]?.GetValue<string>() == "{\"value\":null}", "the send failed: " + done.ToJsonString());
        }
        else Check(started["value"] != null, "the send failed: " + started.ToJsonString());
        Check(Ask(Request(ChatOwner, "chat", "dispose", "")) == "{\"value\":null}", "dispose failed");
    }

    private static void CheckStream()
    {
        var described = JsonNode.Parse(Ask(Request(StreamOwner, "stream", "init", "")))["value"];
        var stream = Only(described["streams"], "the stream service");
        Check(stream["client"]?.GetValue<string>() == "TicksClient" && stream["path"]?.GetValue<string>() == "/ticks", "the stream client was not described: " + described.ToJsonString());
        Check(stream["socket"]?.GetValue<bool>() == false && stream["event"]?.GetValue<string>() == "Tick", "the stream kind was lost: " + described.ToJsonString());
        var key = Only(described["listeners"], "the stream service")["key"].GetValue<string>();
        Check(Ask(Request(StreamOwner, "stream", "event", ",\"name\":\"" + key + "\",\"value\":{\"sku\":\"A\",\"level\":7}")) == "{\"value\":null}", "the listener refused a tick");
        var ticks = Ask(Request(StreamOwner, "stream", "get", ",\"name\":\"" + Only(described["fields"], "the stream service").GetValue<string>() + "\""));
        Check(ticks.Contains("7"), "the listener did not keep the tick: " + ticks);
        Check(Ask(Request(StreamOwner, "stream", "dispose", "")) == "{\"value\":null}", "dispose failed");
    }

    private static JsonNode Only(JsonNode list, string owner)
    {
        Check(list is JsonArray items && items.Count == 1, owner + " described " + (list?.ToJsonString() ?? "nothing"));
        return list[0];
    }

    private static JsonNode Next()
    {
        Check(Signals.TryTake(out var text, TimeSpan.FromSeconds(30)), "no signal arrived");
        return JsonNode.Parse(text);
    }

    private static string Request(string owner, string service, string operation, string rest) =>
        "{\"route\":\"" + Route + "\",\"service\":\"" + service + "\",\"owner\":\"" + owner + "\",\"op\":\"" + operation + "\"" + rest + "}";

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
