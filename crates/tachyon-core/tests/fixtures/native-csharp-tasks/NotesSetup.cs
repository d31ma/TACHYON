[Setup]
public sealed class NotesSetup
{
    public static BehaviorAdapters configure(BehaviorExecution execution)
    {
        execution.Check();
        return new BehaviorAdapters();
    }
}
