public static class TaskSignals
{
    private static readonly Dictionary<string, System.Threading.Tasks.TaskCompletionSource<bool>> Gates = new();
    private static readonly Dictionary<string, System.Threading.Tasks.TaskCompletionSource<bool>> Completions = new();
    public static string Reentry;
    public static string Reentered;

    public static System.Threading.Tasks.Task Hold(string key) => Gate(Gates, key).Task;

    public static void Release(string key) => Gate(Gates, key).TrySetResult(true);

    public static void Finished(string key) => Gate(Completions, key).TrySetResult(true);

    public static System.Threading.Tasks.Task Completed(string key) => Gate(Completions, key).Task;

    public static void Reenter()
    {
        if (Reentry != null) Reentered = Tac.Invoke(Reentry);
    }

    private static System.Threading.Tasks.TaskCompletionSource<bool> Gate(Dictionary<string, System.Threading.Tasks.TaskCompletionSource<bool>> values, string key)
    {
        lock (values)
        {
            if (!values.TryGetValue(key, out var gate))
            {
                gate = new(System.Threading.Tasks.TaskCreationOptions.RunContinuationsAsynchronously);
                values[key] = gate;
            }
            return gate;
        }
    }
}
