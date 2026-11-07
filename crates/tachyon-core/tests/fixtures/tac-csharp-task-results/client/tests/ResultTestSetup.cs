[TestSetup]
public sealed class ResultTestSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        return new BehaviorAdapters();
    }
}
