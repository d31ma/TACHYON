[Setup]
[WhenProfile("development", "test")]
[WhenSetting({"name":"YON_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
public sealed class AnnotatedConditionalConfigSetup {
    public static int calls = 0;
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        calls++;
        return Config.configure(execution);
    }
}

