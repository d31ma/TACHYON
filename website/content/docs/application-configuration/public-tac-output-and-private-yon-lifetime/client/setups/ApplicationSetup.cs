[Setup]
public sealed class ApplicationSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        execution.Check();
        return new BehaviorAdapters();
    }
}
