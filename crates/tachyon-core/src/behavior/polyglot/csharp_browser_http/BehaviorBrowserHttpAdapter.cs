internal sealed class BehaviorBrowserHttpAdapter : BehaviorHTTPAdapter, System.IDisposable
{
    private const int RequestLimit = 65536;
    private const int ResponseLimit = 1048576;
    private readonly System.Func<string, System.Threading.CancellationToken, System.Threading.Tasks.Task<string>> send;
    private readonly System.IDisposable watch;
    private static readonly System.Collections.Generic.HashSet<string> Codes = new(System.StringComparer.Ordinal)
    {
        "HTTP_REQUEST", "HTTP_PATH", "HTTP_ORIGIN", "HTTP_HEADER", "HTTP_QUERY", "HTTP_BODY", "HTTP_LIMIT", "HTTP_RESPONSE",
        "HTTP_STATUS", "HTTP_CAPACITY", "HTTP_NETWORK", "HTTP_MISSING", "CANCELLED", "DEADLINE",
    };

    internal BehaviorBrowserHttpAdapter(System.Func<string, System.Threading.CancellationToken, System.Threading.Tasks.Task<string>> send, System.IDisposable watch) { this.send = send; this.watch = watch; }

    public void Dispose() => watch?.Dispose();

    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) => Exchange(request, execution);

    private async System.Threading.Tasks.Task<BehaviorJSON> Exchange(BehaviorJSON request, BehaviorExecution execution)
    {
        execution.Check();
        request.Bound(RequestLimit);
        var text = await send(request.ToString(), execution.Signal.Token);
        execution.Check();
        return Read(text);
    }

    private static BehaviorJSON Read(string text)
    {
        RequireSize(text);
        return Response(Parse(text));
    }

    private static void RequireSize(string text)
    {
        if (text == null || System.Text.Encoding.UTF8.GetByteCount(text) > ResponseLimit) throw new BehaviorError("HTTP_LIMIT");
    }

    private static BehaviorJSON Parse(string text)
    {
        try { return BehaviorJSON.Parse(text); }
        catch { throw new BehaviorError("HTTP_RESPONSE"); }
    }

    private static BehaviorJSON Response(BehaviorJSON answer)
    {
        var code = Code(answer.Get("error").Get("code").Value.ToString());
        return Select(answer, code);
    }

    private static BehaviorJSON Select(BehaviorJSON answer, string code)
    {
        switch (answer.Has("error"), answer.Has("value"))
        {
            case (true, _): throw new BehaviorError(code);
            case (false, true): return answer.Get("value");
            default: throw new BehaviorError("HTTP_RESPONSE");
        }
    }

    private static string Code(string value) => Codes.Contains(value) ? value : "HTTP_NETWORK";
}
