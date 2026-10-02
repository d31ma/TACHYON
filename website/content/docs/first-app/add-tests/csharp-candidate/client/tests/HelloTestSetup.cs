[TestSetup]
public sealed class HelloTestSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingFakeHelper();
        return adapters;
    }
}
