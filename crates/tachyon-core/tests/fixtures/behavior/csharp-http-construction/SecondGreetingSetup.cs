[Setup]
public sealed class SecondGreetingSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswer(2);
        return adapters;
    }
}

