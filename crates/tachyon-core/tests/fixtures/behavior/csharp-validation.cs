[Checked]
public sealed class AnnotatedInput {
    [NotEmpty]
    [Length({"min":2,"max":8})]
    public BehaviorJSON name;
    [Required]
    [Min(1)]
    [Max(10)]
    public BehaviorJSON amount;
    [Length({"min":1,"max":2})]
    public BehaviorJSON optional;
    private AnnotatedInput() {
        throw new System.Exception("must not instantiate");
    }
}
public sealed class AnnotatedValidationService {
    public int calls = 0;
    public BehaviorJSON accept([Check(typeof(AnnotatedInput))] BehaviorJSON input) {
        calls++;
        return input;
    }
}
[Checked]
public sealed class AnnotatedPageInput {
    [Required]
    [Min(1)]
    public BehaviorJSON size;
}
[CacheSettings({"name":"products","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]
public sealed class AnnotatedValidatedCache {
    [Needs]
    public BehaviorContext behaviors;
    public int calls = 0;
    [Cached]
    public async System.Threading.Tasks.Task<BehaviorJSON> accept([Check(typeof(AnnotatedInput))] BehaviorJSON input) {
        await System.Threading.Tasks.Task.CompletedTask;
        calls++;
        return input;
    }
    public BehaviorJSON page([Check(typeof(AnnotatedPageInput))] [PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})] BehaviorJSON input) {
        calls++;
        return input;
    }
    [WebCall({"client":"catalog","method":"GET","path":"/products"})]
    public async System.Threading.Tasks.Task<BehaviorJSON> remote([Check(typeof(AnnotatedInput))] BehaviorJSON input) {
        await System.Threading.Tasks.Task.CompletedTask;
        calls++;
        return BehaviorJSON.Parse("{}");
    }
}
public static class ValidationAnnotationTests {
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
        CheckRejectionViolations(service);
    }
    private static void CheckRejectionViolations(AnnotatedValidationService service) {
        var rejections = new[] {
            ("{\"amount\":\"1\",\"optional\":7}", "amount.min", "[{\"field\":\"amount\",\"rule\":\"Min\"},{\"field\":\"amount\",\"rule\":\"Max\"},{\"field\":\"name\",\"rule\":\"NotEmpty\"},{\"field\":\"optional\",\"rule\":\"Length\"}]"),
            ("[]", "$.object", "[{\"field\":\"\",\"rule\":\"Checked\"}]"),
        };
        foreach (var (json, path, violations) in rejections) {
            try {
                service.accept(BehaviorJSON.Parse(json));
                throw new System.Exception("invalid accepted");
            } catch (BehaviorError error) when (error.Code == "validation") {
                if (error.Path != path || error.Data["violations"] as string != violations) throw new System.Exception("wrong rejection " + error.Path + " " + error.Data["violations"]);
            }
        }
        try {
            Behaviors.validate(typeof(AnnotatedInput), BehaviorJSON.Parse("{}"));
            throw new System.Exception("invalid accepted");
        } catch (BehaviorError error) when (error.Code == "validation" && error.Path == "amount.notNull") {
            if (error.Data.Contains("violations")) throw new System.Exception("explicit validation reported violations");
        }
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
            Behaviors.validate(typeof(AnnotatedConfig), valid);
            throw new System.Exception("unknown schema accepted");
        } catch (BehaviorError error) when (error.Code == "validation" && error.Path == "$.schema") { }
    }
    private static async System.Threading.Tasks.Task CheckCachedValidation() {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(AnnotatedConfig), "tenant-validation", signal);
        var cached = new AnnotatedValidatedCache(context);
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
            await cached.remote(BehaviorJSON.Parse("{}"));
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
