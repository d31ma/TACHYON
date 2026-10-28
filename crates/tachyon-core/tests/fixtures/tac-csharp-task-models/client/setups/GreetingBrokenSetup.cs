[Setup]
public sealed class GreetingBrokenSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswerDelegate(BehaviorJSON.Parse("{}"));
        return adapters;
    }
}
