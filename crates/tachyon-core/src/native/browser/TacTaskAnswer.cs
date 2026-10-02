internal static class TacTaskAnswer
{
    internal static Func<Exception, string> FailureCode;
    private static readonly HashSet<string> Codes = new(StringComparer.Ordinal)
    {
        "HTTP_REQUEST", "HTTP_PATH", "HTTP_ORIGIN", "HTTP_HEADER", "HTTP_QUERY", "HTTP_BODY", "HTTP_LIMIT", "HTTP_RESPONSE",
        "HTTP_STATUS", "HTTP_CAPACITY", "HTTP_NETWORK", "HTTP_MISSING", "CANCELLED", "DEADLINE",
    };

    internal static System.Threading.Tasks.Task<string> Run(Func<System.Threading.Tasks.Task<string>> work) => Sanitize(work);

    private static async System.Threading.Tasks.Task<string> Sanitize(Func<System.Threading.Tasks.Task<string>> work)
    {
        try { return await work(); }
        catch (System.OperationCanceledException) { return "{\"error\":\"Web service task was cancelled.\",\"code\":\"CANCELLED\"}"; }
        catch (Exception error) { return Failure(error); }
    }
    private static string Failure(Exception error) => Envelope(KnownCode(FailureCode?.Invoke(error)));

    private static string KnownCode(string code) => Codes.Contains(code) ? code : null;

    private static string Envelope(string code) => code == null
        ? "{\"error\":\"Web service task failed.\"}"
        : "{\"error\":\"Web service task failed.\",\"code\":\"" + code + "\"}";
}
