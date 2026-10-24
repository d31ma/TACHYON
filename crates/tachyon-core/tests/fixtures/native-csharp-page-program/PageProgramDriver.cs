using System;
using System.Runtime.InteropServices;
using System.Text.Json.Nodes;

public static class PageProgramDriver
{
    private const string WriterOwner = "00000000-0000-4000-8000-000000000001";
    private const string ReaderOwner = "00000000-0000-4000-8000-000000000002";
    private static readonly string[] Services = { "writer", "reader" };
    private static readonly string[] Fields = { "identity", "count" };
    private static readonly System.Collections.Concurrent.BlockingCollection<string> Signals = new();

    public static void Main()
    {
        Connect();
        foreach (var service in Services) Console.WriteLine(service + " init " + Ask(Request(service, "init", "")));
        Finish("writer", "increment");
        Finish("reader", "inspect");
        foreach (var service in Services)
        {
            foreach (var field in Fields)
            {
                Console.WriteLine(service + " " + field + " " + Ask(Request(service, "get", ",\"name\":\"" + field + "\"")));
            }
        }
        Ask("{\"op\":\"host-dispose\"}");
    }

    private static unsafe void Connect() =>
        ((delegate* unmanaged<IntPtr, void>)&TacNative.NativeSetEmit)((IntPtr)(delegate* unmanaged<IntPtr, void>)&Receive);

    [UnmanagedCallersOnly]
    private static void Receive(IntPtr payload) => Signals.Add(Marshal.PtrToStringUTF8(payload));

    private static void Finish(string service, string name)
    {
        var answer = Ask(Request(service, "call", ",\"name\":\"" + name + "\",\"args\":[]"));
        Console.WriteLine(service + " " + name + " " + answer);
        if (JsonNode.Parse(answer)?["task"] == null) return;
        if (!Signals.TryTake(out var signal, TimeSpan.FromSeconds(30))) throw new Exception("no done signal arrived for " + service);
        Console.WriteLine(service + " " + name + " done " + signal);
    }

    private static string Request(string service, string operation, string rest) =>
        "{\"route\":\"/\",\"service\":\"" + service + "\",\"owner\":\"" + (service == "writer" ? WriterOwner : ReaderOwner) + "\",\"op\":\"" + operation + "\"" + rest + "}";

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
}
