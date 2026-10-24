using System;
using System.Runtime.InteropServices;

public static class NativeListenerDriver
{
    private const string Owner = "00000000-0000-4000-8000-000000000002";
    private const string Route = "/heard";
    private const string Changed = "{\"route\":\"/heard\",\"service\":\"heard\",\"owner\":\"" + Owner + "\",\"changed\":true}";
    private static readonly System.Collections.Concurrent.ConcurrentQueue<string> Signals = new();

    [UnmanagedCallersOnly]
    private static void Receive(IntPtr signal) => Signals.Enqueue(Marshal.PtrToStringUTF8(signal));

    public static unsafe void Main()
    {
        ((delegate* unmanaged<IntPtr, void>)&TacNative.NativeSetEmit)((IntPtr)(delegate* unmanaged<IntPtr, void>)&Receive);
        var described = Ask(Request("init", ""));
        Check(described.Contains("\"task\":true") && described.Contains("\"task\":false"), "the listeners were not described as one task and one plain listener: " + described);
        Check(Ask(Request("call", ",\"name\":\"send\",\"args\":[1]")) == "{\"value\":null}", "the first send failed");
        Check(Ask(Request("call", ",\"name\":\"send\",\"args\":[2]")) == "{\"value\":null}", "the second send failed");
        var heard = WaitFor("a1s1a2s2");
        Check(heard == "{\"value\":\"a1s1a2s2\"}", "the listeners did not run one after another in source order: " + heard);
        Check(WaitForChanges(2), "a finished listener task did not tell the page to read the service again: " + string.Join(" ", Signals));
        Check(Ask(Request("call", ",\"name\":\"send\",\"args\":[3]")) == "{\"value\":null}", "the third send failed");
        Check(Ask(Request("dispose", "")) == "{\"value\":null}", "dispose failed");
        System.Threading.Thread.Sleep(100);
        Ask("{\"op\":\"host-dispose\"}");
        Console.WriteLine("PASS native C# async listeners run in order");
    }

    private static string WaitFor(string wanted)
    {
        var answer = "";
        for (var attempt = 0; attempt < 300; attempt++)
        {
            answer = Ask(Request("get", ",\"name\":\"heard\""));
            if (answer == "{\"value\":\"" + wanted + "\"}") return answer;
            System.Threading.Thread.Sleep(10);
        }
        return answer;
    }

    private static bool WaitForChanges(int wanted)
    {
        for (var attempt = 0; attempt < 300; attempt++)
        {
            if (System.Linq.Enumerable.Count(Signals, signal => signal == Changed) >= wanted) return true;
            System.Threading.Thread.Sleep(10);
        }
        return false;
    }

    private static string Request(string operation, string rest) =>
        "{\"route\":\"" + Route + "\",\"service\":\"heard\",\"owner\":\"" + Owner + "\",\"op\":\"" + operation + "\"" + rest + "}";

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
