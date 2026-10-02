[Setup]
public sealed class GreetingSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        return new BehaviorAdapters();
    }
}

