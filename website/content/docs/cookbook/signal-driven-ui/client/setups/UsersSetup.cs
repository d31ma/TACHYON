[Setup]
public sealed class UsersSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        execution.Check();
        return new BehaviorAdapters();
    }
}
