[Setup]
public sealed class BrowserHttpSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution) => HttpProbeBindings.Configuration(false);
}
