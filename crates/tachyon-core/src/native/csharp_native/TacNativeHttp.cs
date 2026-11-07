internal static class TacNativeHttp
{
    private const int PendingLimit = 64;
    private static readonly System.Net.Http.HttpClient Client = CreateClient();
    private static readonly Dictionary<string, TacNativeHttpExchange> Exchanges = new(StringComparer.Ordinal);
    private static int Pending;

    internal static string Start(string text)
    {
        if (TacApplication.YonOrigin.Length == 0) return TacNativeHttpFailure.Answer("HTTP_MISSING");
        TacNativeHttpRequest request;
        try { request = TacNativeHttpRequest.Read(text, TacApplication.YonOrigin); }
        catch (TacNativeHttpFailure failure) { return failure.Envelope(); }
        catch (Exception) { return TacNativeHttpFailure.Answer("HTTP_REQUEST"); }
        return Begin(request);
    }

    internal static System.Threading.Tasks.Task<string> Answer(string ticket)
    {
        lock (Exchanges)
        {
            if (Exchanges.TryGetValue(ticket, out var exchange)) return exchange.Answer;
        }
        return System.Threading.Tasks.Task.FromResult(TacNativeHttpFailure.Answer("HTTP_NETWORK"));
    }

    internal static void Cancel(string ticket)
    {
        TacNativeHttpExchange exchange;
        lock (Exchanges)
        {
            if (!Exchanges.Remove(ticket, out exchange)) return;
        }
        exchange.Cancel();
    }

    private static string Begin(TacNativeHttpRequest request)
    {
        var ticket = Guid.NewGuid().ToString("D");
        lock (Exchanges)
        {
            if (Pending >= PendingLimit) return TacNativeHttpFailure.Answer("HTTP_CAPACITY");
            Pending++;
            Exchanges[ticket] = TacNativeHttpExchange.Begin(Client, request, Finished);
        }
        return ticket;
    }

    private static void Finished()
    {
        lock (Exchanges) { Pending--; }
    }

    private static System.Net.Http.HttpClient CreateClient()
    {
        var handler = new System.Net.Http.SocketsHttpHandler
        {
            AllowAutoRedirect = false,
            UseCookies = false,
            AutomaticDecompression = System.Net.DecompressionMethods.None,
        };
        return new System.Net.Http.HttpClient(handler) { Timeout = System.Threading.Timeout.InfiniteTimeSpan };
    }
}
