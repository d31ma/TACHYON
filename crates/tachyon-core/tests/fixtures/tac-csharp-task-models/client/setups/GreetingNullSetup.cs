[Setup]
public sealed class GreetingNullSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswerDelegate(BehaviorJSON.Parse("{\"message\":null}"));
        return adapters;
    }
}
