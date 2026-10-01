public static class WindowExampleCheck
{
    public static void Main()
    {
        var operations = new List<string>();
        NativeCapabilities.Call = (operation, request) => {
            operations.Add(operation);
            return operation == "window.granted" ? "{\"granted\":true}" : "{\"ok\":true}";
        };
        var prefix = "{\"route\":\"/\",\"owner\":\"6F9619FF-8B86-D011-B42D-00C04FC964FF\",\"service\":\"report\",";
        var started = Tac.Invoke(prefix + "\"op\":\"init\"}");
        if (started.Contains("error") || started.Contains("windowControls")) throw new Exception(started);
        foreach (var method in new[] { "minimize", "resize", "rename" })
        {
            var answer = Tac.Invoke(prefix + "\"op\":\"call\",\"name\":\"" + method + "\",\"args\":[]}");
            if (answer.Contains("error")) throw new Exception(answer);
        }
        foreach (var operation in new[] { "window.minimize", "window.resize", "window.title" })
        {
            if (!operations.Contains(operation)) throw new Exception("Window provider did not call " + operation);
        }
        Tac.Invoke(prefix + "\"op\":\"dispose\"}");
        Console.WriteLine("C# website ReportService executed with the captured injected provider; no Windows OS runtime claim");
    }
}
