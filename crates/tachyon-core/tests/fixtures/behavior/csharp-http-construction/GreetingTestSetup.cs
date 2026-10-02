[TestSetup]
public sealed class GreetingTestSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        return new BehaviorAdapters();
    }
}

