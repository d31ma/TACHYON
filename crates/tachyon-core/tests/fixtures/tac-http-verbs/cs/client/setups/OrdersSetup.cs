[Setup]
public sealed class OrdersSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        return new BehaviorAdapters();
    }
}
