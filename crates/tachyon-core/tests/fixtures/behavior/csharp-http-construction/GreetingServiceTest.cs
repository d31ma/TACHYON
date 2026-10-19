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

    void testUnassignedFakeGetsAClientOnTheTestContext() {
        greetingClient = null;
        if (construct<GreetingService>() == null) throw new System.Exception("Unassigned Fake did not get a client.");
    }
}

