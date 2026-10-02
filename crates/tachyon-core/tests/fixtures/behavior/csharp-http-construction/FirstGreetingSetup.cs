[Setup]
public sealed class FirstGreetingSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswer(1);
        return adapters;
    }
}

