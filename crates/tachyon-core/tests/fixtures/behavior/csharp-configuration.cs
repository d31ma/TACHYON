[Settings({"prefix":"TAC_APP_"})]
public sealed class AnnotatedSettings {
    public string title;
    [Min(1)]
    [Max(100)]
    public int size = 20;
    private AnnotatedSettings() {
        throw new System.Exception("must not instantiate");
    }
}
public sealed class AnnotatedValues {
    [Setting("${tac.app.title:Untitled}")]
    public string title = "";
    [Setting({"path":"tac.app.size","type":"integer","default":5,"min":1,"max":100})]
    public long size = 0;
    [Setting("${tac.app.title:Untitled}")]
    public string configuredApplicationValuexxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx = "";
}
[Setup]
[WhenProfile("development", "test")]
[WhenSetting({"name":"TAC_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]
public sealed class AnnotatedConditionalConfig {
    public static int calls = 0;
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        calls++;
        return Config.configure(execution);
    }
}
public static class ConfigurationAnnotationTests {
    private static void CheckBinding() {
        var values = new System.Collections.Generic.Dictionary<string,string> { ["TAC_APP_TITLE"] = "Products" };
        var bound = Behaviors.properties(typeof(AnnotatedSettings), values);
        if (bound.Get("title").Text != "Products" || bound.Get("size").Integer != 20) throw new System.Exception("generated properties failed");
        var application = Behaviors.values(typeof(AnnotatedValues), BehaviorJSON.Parse("{\"tac\":{\"app\":{\"size\":12}}}"));
        if (application.Get("title").Text != "Untitled" || application.Get("size").Integer != 12) throw new System.Exception("generated values failed");
        if (application.Get("configuredApplicationValuexxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").Text != "Untitled") throw new System.Exception("128-character Value field failed");
        try {
            Behaviors.values(typeof(AnnotatedValues), BehaviorJSON.Parse("{\"secrets\":{}}"));
            throw new System.Exception("unknown section accepted");
        } catch (BehaviorError error) when (error.Code == "configuration-properties") { }
        var manifest = Behaviors.values(typeof(AnnotatedValues), BehaviorJSON.Parse("{\"name\":\"Catalog\",\"icons\":[{\"src\":\"/icon.svg\"}],\"tac\":{\"app\":{\"size\":7}}}"));
        if (manifest.Get("size").Integer != 7) throw new System.Exception("manifest members were not ignored");
    }
    private static void CheckConditionalConfiguration() {
        using var signal = new BehaviorSignal();
        try {
            using var unexpected = Behaviors.create(typeof(AnnotatedConditionalConfig), "tenant-one", signal);
            throw new System.Exception("inactive accepted");
        } catch (BehaviorError error) when (error.Code == "inactive-configuration") { }
        if (AnnotatedConditionalConfig.calls != 0) throw new System.Exception("inactive factory called");
        using var active = Behaviors.create(typeof(AnnotatedConditionalConfig), "tenant-one", signal, profiles: new[] { "test" }, properties: new System.Collections.Generic.Dictionary<string,string> { ["TAC_CACHE_ENABLED"] = "true" });
        if (AnnotatedConditionalConfig.calls != 1) throw new System.Exception("active factory not called");
    }
    public static void Run() {
        CheckBinding();
        CheckConditionalConfiguration();
    }
}
