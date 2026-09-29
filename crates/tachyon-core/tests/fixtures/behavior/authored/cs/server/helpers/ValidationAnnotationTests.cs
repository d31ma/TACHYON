[Description("Behavior conformance ValidationAnnotationTests")]
public class ValidationAnnotationTests {
    private static void CheckServiceValidation(BehaviorJSON valid) {
        var service = new AnnotatedValidationService();
        if (Behaviors.validate(typeof(AnnotatedInput), valid).ToString() != valid.ToString() || service.accept(valid).ToString() != valid.ToString()) throw new System.Exception("validation mutated value");
        var invalidInputs = new[] {
            ("{}", "amount.notNull"),
            ("{\"name\":\"é\",\"amount\":0}", "amount.min"),
            ("{\"name\":\"é\",\"amount\":11}", "amount.max"),
            ("{\"name\":\"é\",\"amount\":\"1\"}", "amount.min"),
            ("{\"name\":\"x\",\"amount\":1}", "name.size"),
            ("{\"name\":\"é\",\"amount\":1,\"optional\":[]}", "optional.size"),
            ("[]", "$.object"),
        };
        foreach (var (json, path) in invalidInputs) {
            try {
                service.accept(BehaviorJSON.Parse(json));
                throw new System.Exception("invalid accepted");
            } catch (BehaviorError error) when (error.Code == "validation") {
                if (error.Path != path) throw new System.Exception("wrong validation path " + error.Path);
            }
        }
        foreach (var whitespace in new[] { "", "\t\n\r ", "\u0085\u00a0\u1680\u2000\u200a\u2028\u2029\u202f\u205f\u3000" }) {
            try {
                var fields = new System.Collections.Generic.Dictionary<string,BehaviorJSON> { ["name"] = BehaviorJSON.From(whitespace), ["amount"] = BehaviorJSON.From(1) };
                service.accept(BehaviorJSON.Object(fields));
                throw new System.Exception("blank accepted");
            } catch (BehaviorError error) when (error.Code == "validation" && error.Path == "name.notBlank") { }
        }
        if (service.calls != 1) throw new System.Exception("invalid method ran");
    }
    private static void CheckDirectValidation(BehaviorJSON valid) {
        try {
            Behaviors.validate(typeof(AnnotatedInput), null);
            throw new System.Exception("null accepted");
        } catch (BehaviorError error) when (error.Code == "validation" && error.Path == "$.json") { }
        Behaviors.validate(typeof(AnnotatedInput), BehaviorJSON.Parse("{\"name\":\"é\",\"amount\":1,\"optional\":null}"));
        Behaviors.validate(typeof(AnnotatedInput), BehaviorJSON.Parse("{\"name\":\"é\",\"amount\":1,\"optional\":{\"a\":true}}"));
        Behaviors.validate(typeof(AnnotatedInput), BehaviorJSON.Parse("{\"name\":\"\u200b\",\"amount\":1}"));
        try {
            Behaviors.validate(typeof(AnnotatedConfigSetup), valid);
            throw new System.Exception("unknown schema accepted");
        } catch (BehaviorError error) when (error.Code == "validation" && error.Path == "$.schema") { }
    }
    private static async System.Threading.Tasks.Task CheckCachedValidation() {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(AnnotatedConfigSetup), "tenant-validation", signal);
        var cached = new AnnotatedValidatedCacheService { behaviors = context };
        var client = new AnnotatedValidatedCacheClient { behaviors = context };
        int authorizations = Config.Authorizations;
        int reads = Config.Store.Reads;
        try {
            await cached.accept(BehaviorJSON.Parse("{}"));
            throw new System.Exception("invalid cache input accepted");
        } catch (BehaviorError error) when (error.Code == "validation") { }
        if (cached.calls != 0 || authorizations != Config.Authorizations || reads != Config.Store.Reads) throw new System.Exception("invalid cached body or adapter ran");
        try {
            cached.page(BehaviorJSON.Parse("{\"size\":0}"));
            throw new System.Exception("invalid page accepted");
        } catch (BehaviorError error) when (error.Code == "validation" && error.Path == "size.min") { }
        try {
            await client.remote(BehaviorJSON.Parse("{}"));
            throw new System.Exception("invalid HTTP accepted");
        } catch (BehaviorError error) when (error.Code == "validation") { }
        if (cached.calls != 0 || cached.page(BehaviorJSON.Parse("{\"size\":2,\"page\":3}")).Get("offset").Integer != 6) throw new System.Exception("validation before page/HTTP");
    }
    public static async System.Threading.Tasks.Task Run() {
        var valid = BehaviorJSON.Parse("{\"name\":\"é\",\"amount\":1,\"extra\":true}");
        CheckServiceValidation(valid);
        CheckDirectValidation(valid);
        await CheckCachedValidation();
    }
}
