[Service]
[Lifetime({"value":"prototype"})]
[Description("Browser Task continuation conformance")]
public sealed class DelayTaskService
{
    private const int MaximumDelay = 1000;
    private const int ForgetDelay = 20;

    public int Calls = 0;
    public string Last = "ready";

    public System.Threading.Tasks.Task<BehaviorJSON> Load(string key, int milliseconds) => LoadAfterDelay(key, milliseconds);

    private static void RequireDelay(int milliseconds)
    {
        if (milliseconds < 0 || milliseconds > MaximumDelay) throw new System.ArgumentOutOfRangeException(nameof(milliseconds));
    }

    private async System.Threading.Tasks.Task<BehaviorJSON> LoadAfterDelay(string key, int milliseconds)
    {
        RequireDelay(milliseconds);
        Calls++;
        await System.Threading.Tasks.Task.Delay(milliseconds);
        Last = key;
        return BehaviorJSON.Object(new System.Collections.Generic.Dictionary<string, BehaviorJSON>
        {
            ["key"] = BehaviorJSON.From(key),
            ["calls"] = BehaviorJSON.From(Calls),
        });
    }

    public System.Threading.Tasks.Task Forget() => ForgetAfterDelay();

    private async System.Threading.Tasks.Task ForgetAfterDelay()
    {
        Calls++;
        await System.Threading.Tasks.Task.Delay(ForgetDelay);
        Last = "void";
    }

    public System.Threading.Tasks.Task<BehaviorJSON> Failure()
    {
        return System.Threading.Tasks.Task.FromException<BehaviorJSON>(new System.InvalidOperationException("private fixture failure detail"));
    }
}
