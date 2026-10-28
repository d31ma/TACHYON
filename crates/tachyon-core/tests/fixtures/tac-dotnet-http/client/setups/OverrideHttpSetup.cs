[Setup]
public sealed class OverrideHttpSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        var adapters = new BehaviorAdapters();
        adapters.http["probe"] = new CustomHttpDelegate();
        return adapters;
    }
}
