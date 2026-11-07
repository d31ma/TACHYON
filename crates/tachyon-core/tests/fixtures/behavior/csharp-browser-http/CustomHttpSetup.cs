[Setup]
public sealed class CustomHttpSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution) => HttpProbeBindings.Configuration(true);
}
