[Setup]
public sealed class GreetingExtraSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswerHelper(BehaviorJSON.Parse("{\"message\":\"answer\",\"extra\":true}"));
        return adapters;
    }
}
