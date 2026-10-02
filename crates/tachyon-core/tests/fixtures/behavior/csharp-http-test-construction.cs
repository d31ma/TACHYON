[HTTPClient({"path":"/api/greeting","adapter":"greeting"})]
public sealed class GreetingClient {
    [GetRequest]
    public async System.Threading.Tasks.Task<BehaviorJSON> read() {}
}

[Service]
public sealed class GreetingService {
    [Needs]
    private readonly GreetingClient greetingClient;

    public System.Threading.Tasks.Task<BehaviorJSON> read() {
        return greetingClient.read();
    }
}

[TestSetup]
public sealed class GreetingTestSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        return new BehaviorAdapters();
    }
}

[Setup]
public sealed class GreetingSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        return new BehaviorAdapters();
    }
}

[Setup]
public sealed class FirstGreetingSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswer(1);
        return adapters;
    }
}

[Setup]
public sealed class SecondGreetingSetup {
    public static BehaviorAdapters configure(BehaviorExecution execution) {
        var adapters = new BehaviorAdapters();
        adapters.http["greeting"] = new GreetingAnswer(2);
        return adapters;
    }
}

public sealed class GreetingAnswer : BehaviorHTTPAdapter {
    private readonly long value;

    public GreetingAnswer(long value) {
        this.value = value;
    }

    public System.Threading.Tasks.Task<BehaviorJSON> exchange(BehaviorJSON request, BehaviorExecution execution) {
        execution.Check();
        if (request.Get("method").Text != "GET" || request.Get("path").Text != "/api/greeting") {
            throw new System.InvalidOperationException("The actual authored client descriptor was not used.");
        }
        return System.Threading.Tasks.Task.FromResult(BehaviorJSON.From(value));
    }
}

[Test(typeof(GreetingTestSetup))]
public sealed class GreetingServiceTest {
    [Fake]
    public GreetingClient greetingClient = null;

    void testIndependentTypedClients() {
        using var firstSignal = new BehaviorSignal();
        using var firstContext = Behaviors.create(typeof(FirstGreetingSetup), "first", firstSignal);
        using var secondSignal = new BehaviorSignal();
        using var secondContext = Behaviors.create(typeof(SecondGreetingSetup), "second", secondSignal);
        greetingClient = new GreetingClient(firstContext);
        var first = construct<GreetingService>();
        var sameFake = construct<GreetingService>();
        if (object.ReferenceEquals(first, sameFake)) throw new System.Exception("Construction reused a Service.");
        greetingClient = new GreetingClient(secondContext);
        var second = construct<GreetingService>();
        if (first.read().GetAwaiter().GetResult().Integer != 1 || sameFake.read().GetAwaiter().GetResult().Integer != 1 || second.read().GetAwaiter().GetResult().Integer != 2) {
            throw new System.Exception("Construction did not retain its assigned typed client.");
        }
        firstContext.Dispose();
        var refused = false;
        try { first.read().GetAwaiter().GetResult(); }
        catch (BehaviorError) { refused = true; }
        if (!refused) throw new System.Exception("Disposed fake context remained usable.");
        if (second.read().GetAwaiter().GetResult().Integer != 2) throw new System.Exception("One client disposal retired another client.");
    }

    void testUnassignedFakeIsRefused() {
        greetingClient = null;
        var refused = false;
        try { construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { refused = error.Message.Contains("assigned Fake named greetingClient"); }
        if (!refused) throw new System.Exception("Unassigned Fake was not refused.");
    }
}

[Test(typeof(GreetingTestSetup))]
public sealed class MissingGreetingServiceTest {
    void testMissingFakeIsRefused() {
        var refused = false;
        try { construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { refused = error.Message.Contains("matching typed Fake named greetingClient"); }
        if (!refused) throw new System.Exception("Missing Fake was not refused.");
    }
}

[Test(typeof(GreetingTestSetup))]
public sealed class IncompatibleGreetingServiceTest {
    [Fake]
    public string greetingClient = "wrong type";

    void testIncompatibleFakeIsRefused() {
        var refused = false;
        try { construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { refused = error.Message.Contains("matching typed Fake named greetingClient"); }
        if (!refused) throw new System.Exception("Incompatible Fake was not refused.");
    }
}

public static class EmittedBehaviorTests {
    public static void Main() {
        var unit = new GreetingServiceTest();
        var inactiveRefused = false;
        try { unit.construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { inactiveRefused = error.Message.Contains("active test setup"); }
        if (!inactiveRefused) throw new System.Exception("Inactive Test construction was accepted.");
        var reports = new string[] { unit.POST(), new MissingGreetingServiceTest().POST(), new IncompatibleGreetingServiceTest().POST() };
        foreach (var report in reports) {
            var cases = BehaviorJSON.Parse(report).Get("cases").Items;
            if (cases.Length == 0) throw new System.Exception("No authored Test executed.");
            foreach (var result in cases) {
                if (!result.Get("passed").Boolean) throw new System.Exception(result.ToString());
            }
        }
        inactiveRefused = false;
        try { unit.construct<GreetingService>(); }
        catch (System.InvalidOperationException error) { inactiveRefused = error.Message.Contains("active test setup"); }
        if (!inactiveRefused) throw new System.Exception("Stopped Test construction was accepted.");
        System.Console.WriteLine("PASS C# HTTPClient typed Test construction");
    }
}
