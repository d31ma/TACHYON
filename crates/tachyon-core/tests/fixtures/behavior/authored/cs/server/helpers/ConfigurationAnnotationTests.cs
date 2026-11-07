[Description("Behavior conformance ConfigurationAnnotationTests")]
public class ConfigurationAnnotationTests {
    private static void CheckBinding() {
        var values = new System.Collections.Generic.Dictionary<string,string> { ["YON_APP_TITLE"] = "Products" };
        var bound = Behaviors.properties(typeof(AnnotatedSettings), values);
        if (bound.Get("title").Text != "Products" || bound.Get("size").Integer != 20) throw new System.Exception("generated properties failed");
        var application = Behaviors.values(typeof(AnnotatedValues), BehaviorJSON.Parse("{\"yon\":{\"app\":{\"size\":12}}}"));
        if (application.Get("title").Text != "Untitled" || application.Get("size").Integer != 12) throw new System.Exception("generated values failed");
        if (application.Get("configuredApplicationValuexxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx").Text != "Untitled") throw new System.Exception("128-character Value field failed");
        try {
            Behaviors.values(typeof(AnnotatedValues), BehaviorJSON.Parse("{\"secrets\":{}}"));
            throw new System.Exception("unknown section accepted");
        } catch (BehaviorError error) when (error.Code == "configuration-properties") { }
        var manifest = Behaviors.values(typeof(AnnotatedValues), BehaviorJSON.Parse("{\"name\":\"Catalog\",\"icons\":[{\"src\":\"/icon.svg\"}],\"yon\":{\"app\":{\"size\":7}}}"));
        if (manifest.Get("size").Integer != 7) throw new System.Exception("manifest members were not ignored");
    }
    private static void CheckConditionalConfiguration() {
        using var signal = new BehaviorSignal();
        try {
            using var unexpected = Behaviors.create(typeof(AnnotatedConditionalConfigSetup), "tenant-one", signal);
            throw new System.Exception("inactive accepted");
        } catch (BehaviorError error) when (error.Code == "inactive-configuration") { }
        if (AnnotatedConditionalConfigSetup.calls != 0) throw new System.Exception("inactive factory called");
        using var active = Behaviors.create(typeof(AnnotatedConditionalConfigSetup), "tenant-one", signal, profiles: new[] { "test" }, properties: new System.Collections.Generic.Dictionary<string,string> { ["YON_CACHE_ENABLED"] = "true" });
        if (AnnotatedConditionalConfigSetup.calls != 1) throw new System.Exception("active factory not called");
    }
    public static void Run() {
        CheckBinding();
        CheckConditionalConfiguration();
    }
}
