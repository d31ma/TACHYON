internal static class TacHttpTransport
{
    internal static Func<string, string> Start;
    internal static Func<string, System.Threading.Tasks.Task<string>> Answer;
    internal static Action<string> Cancel;
}
