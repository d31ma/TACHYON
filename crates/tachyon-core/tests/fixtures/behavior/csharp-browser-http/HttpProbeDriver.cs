internal static class HttpProbeDriver
{
    public static System.Threading.Tasks.Task Main() => Run();

    private static async System.Threading.Tasks.Task Run()
    {
        BehaviorBrowserHttp.Capture = null;
        using var missing = Behaviors.create(typeof(BrowserHttpSetup), "missing", new BehaviorSignal());
        await Refused(() => missing.restClient("probe").get("/api/probe").retrieve(), "HTTP_MISSING");
        BehaviorBrowserHttp.Capture = HttpProbeBindings.Capture;
        var first = new HttpProbeTransport("first");
        HttpProbeBindings.Current = first;
        using var context = Behaviors.create(typeof(BrowserHttpSetup), "first", new BehaviorSignal());
        var second = new HttpProbeTransport("second");
        HttpProbeBindings.Current = second;
        using var another = Behaviors.create(typeof(BrowserHttpSetup), "second", new BehaviorSignal());
        Check((await context.restClient("probe").get("/api/probe").retrieve()).Get("owner").Text == "first", "Context changed to another factory owner");
        Check((await another.restClient("probe").get("/api/probe").retrieve()).Get("owner").Text == "second", "Context owner leaked");
        Check(!first.Requests[0].Has("publicRead") && !first.Requests[0].Has("revalidate"), "REST became browser-cacheable");
        Check(first.Requests[0].Get("credentials").Text == "omit" && first.Requests[0].Get("redirect").Text == "error", "HTTP envelope changed");
        var cache = new CacheProbeService(context);
        await cache.Read();
        var initial = first.Requests[^1];
        Check(initial.Get("publicRead").Boolean && !initial.Get("revalidate").Boolean, "Cached GET hints missing");
        var count = first.Requests.Count;
        await cache.Read();
        Check(first.Requests.Count == count, "Cache hit repeated HTTP");
        await cache.Clear();
        await cache.Read();
        Check(first.Requests[^1].Get("revalidate").Boolean, "Cache clear did not revalidate");
        await context.evict("CacheProbeService", "Read");
        await cache.Read();
        Check(first.Requests[^1].Get("revalidate").Boolean, "Explicit eviction did not revalidate");
        var otherCache = new CacheProbeService(another);
        await otherCache.Read();
        Check(!second.Requests[^1].Get("revalidate").Boolean, "Revalidation crossed contexts");
        HttpProbeBindings.Current = new HttpProbeTransport("unused");
        using var custom = Behaviors.create(typeof(CustomHttpSetup), "custom", new BehaviorSignal());
        var customCache = new CacheProbeService(custom);
        Check((await customCache.Read()).Get("owner").Text == "custom", "Custom Setup lost precedence");
        Check(!HttpProbeBindings.Override.Requests[^1].Has("publicRead") && !HttpProbeBindings.Override.Requests[^1].Has("revalidate"), "Custom adapter descriptor changed");
        Check(HttpProbeBindings.Current.Requests.Count == 0, "Custom adapter invoked default host");
        first.Response = "{\"error\":{\"code\":\"HTTP_STATUS\"}}";
        await Refused(() => context.restClient("probe").get("/api/probe").retrieve(), "HTTP_STATUS");
        first.Response = "{\"error\":{\"code\":\"secret invalid code\"}}";
        await Refused(() => context.restClient("probe").get("/api/probe").retrieve(), "HTTP_NETWORK");
        first.Response = "invalid JSON";
        await Refused(() => context.restClient("probe").get("/api/probe").retrieve(), "HTTP_RESPONSE");
        first.Response = new string('x', 1048577);
        await Refused(() => context.restClient("probe").get("/api/probe").retrieve(), "HTTP_LIMIT");
        first.Response = null;
        first.Mode = "wait";
        var pending = context.restClient("probe").get("/api/slow").retrieve();
        first.Lifetime.Cancel();
        await Refused(() => pending, "CANCELLED");
        await Refused(() => context.restClient("probe").get("/api/probe").retrieve(), "CANCELLED");
        Check((await another.restClient("probe").get("/api/probe").retrieve()).Get("owner").Text == "second", "Disposal cancelled another owner");
        CheckSetupRelease();
        var marks = new BehaviorHttpRevalidation();
        for (int index = 0; index <= 4096; index++) marks.Mark(index.ToString());
        Check(marks.With("not-stored", BehaviorJSON.Parse("{}")).Get("_httpRevalidate").Boolean, "Overflow lost conservative revalidation");
        marks.Clear();
        Check(!marks.With("not-stored", BehaviorJSON.Parse("{}")).Get("_httpRevalidate").Boolean, "Released marks retained state");
        System.Console.WriteLine("PASS C# owned browser HTTP Context/cache prerequisite");
    }

    private static void CheckSetupRelease()
    {
        var calls = HttpProbeBindings.ConfigurationCalls;
        var expired = new HttpProbeTransport("expired-construction");
        expired.Lifetime.Cancel();
        HttpProbeBindings.Current = expired;
        using var partial = Behaviors.create(typeof(BrowserHttpSetup), "partial", new BehaviorSignal(), profiles: new[] { "cleanup" });
        Check(HttpProbeBindings.ConfigurationCalls == calls + 1, "Expired construction did not acquire Setup");
        partial.dispose();
        HttpProbeBindings.Current = new HttpProbeTransport("replacement");
        using var replacement = Behaviors.create(typeof(BrowserHttpSetup), "replacement", new BehaviorSignal(), profiles: new[] { "cleanup" });
        Check(HttpProbeBindings.ConfigurationCalls == calls + 2, "Expired construction leaked Setup ownership");
        partial.dispose();
        using var shared = Behaviors.create(typeof(BrowserHttpSetup), "shared", new BehaviorSignal(), profiles: new[] { "cleanup" });
        Check(HttpProbeBindings.ConfigurationCalls == calls + 2, "Repeated disposal released another Context's Setup");
        replacement.dispose();
        replacement.dispose();
        shared.dispose();
        using var fresh = Behaviors.create(typeof(BrowserHttpSetup), "fresh", new BehaviorSignal(), profiles: new[] { "cleanup" });
        Check(HttpProbeBindings.ConfigurationCalls == calls + 3, "Final disposal did not release Setup exactly once");
    }

    private static async System.Threading.Tasks.Task Refused(System.Func<System.Threading.Tasks.Task<BehaviorJSON>> work, string code)
    {
        try { await work(); }
        catch (BehaviorError error) { Check(error.Code == code, "Wrong safe HTTP failure: " + error.Code); return; }
        catch (System.OperationCanceledException) { Check(code == "CANCELLED", "Unexpected cancellation"); return; }
        throw new System.Exception("Missing expected refusal " + code);
    }

    private static void Check(bool condition, string message)
    {
        if (!condition) throw new System.Exception(message);
    }
}
