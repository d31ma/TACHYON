[Setup]
public sealed class CollectionFakeSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["collections"] = new CollectionAnswerDelegate();
        return adapters;
    }
}
