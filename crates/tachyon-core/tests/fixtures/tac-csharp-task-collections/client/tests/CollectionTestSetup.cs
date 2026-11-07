[TestSetup]
public sealed class CollectionTestSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        return new BehaviorAdapters();
    }
}
