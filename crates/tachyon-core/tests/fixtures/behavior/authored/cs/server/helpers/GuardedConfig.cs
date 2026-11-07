[Description("Behavior conformance GuardedConfig")]
public sealed class GuardedConfig {
    public static int Calls;
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        Calls++;
        return Config.configure(execution);
    }
}

