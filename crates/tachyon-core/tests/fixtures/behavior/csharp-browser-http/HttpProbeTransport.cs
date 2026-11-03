internal sealed class HttpProbeTransport
{
    internal readonly string Name;
    internal readonly System.Threading.CancellationTokenSource Lifetime = new();
    internal readonly System.Collections.Generic.List<BehaviorJSON> Requests = new();
    internal string Mode = "success";
    internal string Response;

    internal HttpProbeTransport(string name) { Name = name; }

    internal async System.Threading.Tasks.Task<string> Send(string text, System.Threading.CancellationToken signal)
    {
        Requests.Add(BehaviorJSON.Parse(text));
        using var linked = System.Threading.CancellationTokenSource.CreateLinkedTokenSource(signal, Lifetime.Token);
        if (Mode == "wait") await System.Threading.Tasks.Task.Delay(1000, linked.Token);
        linked.Token.ThrowIfCancellationRequested();
        return Response ?? "{\"value\":{\"owner\":\"" + Name + "\"}}";
    }
}
