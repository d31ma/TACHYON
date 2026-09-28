[Description("Behavior conformance MissingConfig")]
public sealed class MissingConfig {
    public static int Calls;
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        Calls++;
        return Config.configure(execution);
    }
}

