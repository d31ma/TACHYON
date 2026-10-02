internal static class TacTaskAnswer
{
    internal static System.Threading.Tasks.Task<string> Run(Func<System.Threading.Tasks.Task<string>> work) => Sanitize(work);

    private static async System.Threading.Tasks.Task<string> Sanitize(Func<System.Threading.Tasks.Task<string>> work)
    {
        try { return await work(); }
        catch (System.OperationCanceledException) { return "{\"error\":\"Web service task was cancelled.\",\"code\":\"CANCELLED\"}"; }
        catch (Exception) { return "{\"error\":\"Web service task failed.\"}"; }
    }
}
