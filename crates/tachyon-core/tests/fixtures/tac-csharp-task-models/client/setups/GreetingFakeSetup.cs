[Setup]
public sealed class GreetingFakeSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswerHelper(BehaviorJSON.Parse("{\"message\":\"typed fake answer\"}"));
        return adapters;
    }
}
