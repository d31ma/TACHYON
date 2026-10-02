[Test(typeof(GreetingTestSetup))]
public sealed class GreetingServiceTest
{
    [Fake]
    public GreetingClient _greetingClient = null;

    void testTypedFakeKeepsModelAndIndependentOwners()
    {
        var nullRefused = false;
        try { new GreetingClient(null); }
        catch (System.ArgumentNullException) { nullRefused = true; }
        Assert.isTrue(nullRefused);
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(GreetingFakeSetup), "typed-fake", signal);
        _greetingClient = new GreetingClient(context);
        var first = construct<GreetingService>();
        var second = construct<GreetingService>();
        var answer = first.ask().GetAwaiter().GetResult();
        Assert.equals("typed fake answer", answer.Message);
        Assert.equals("typed fake answer", first.Message);
        Assert.equals("", second.Message);
        context.Dispose();
        var refused = false;
        try { first.ask().GetAwaiter().GetResult(); }
        catch (BehaviorError error) { refused = error.Code == "CANCELLED"; }
        Assert.isTrue(refused);
    }

    void testMissingRequiredFieldIsRefused()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(GreetingBrokenSetup), "broken-fake", signal);
        _greetingClient = new GreetingClient(context);
        var service = construct<GreetingService>();
        var refused = false;
        try { service.ask().GetAwaiter().GetResult(); }
        catch (BehaviorError error) { refused = error.Code == "HTTP_RESPONSE"; }
        Assert.isTrue(refused);
        Assert.equals("", service.Message);
    }

    void testNullableFieldAndRequiredNullAreDistinct()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(GreetingNullSetup), "nullable-fake", signal);
        _greetingClient = new GreetingClient(context);
        var service = construct<GreetingService>();
        var nullable = service.nullable().GetAwaiter().GetResult();
        Assert.equals(null, nullable.Message);
        var refused = false;
        try { service.ask().GetAwaiter().GetResult(); }
        catch (BehaviorError error) { refused = error.Code == "HTTP_RESPONSE"; }
        Assert.isTrue(refused);
    }

    void testClosedRecordExtraFieldIsRefused()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(GreetingExtraSetup), "extra-fake", signal);
        _greetingClient = new GreetingClient(context);
        var service = construct<GreetingService>();
        var refused = false;
        try { service.ask().GetAwaiter().GetResult(); }
        catch (BehaviorError error) { refused = error.Code == "HTTP_RESPONSE"; }
        Assert.isTrue(refused);
        Assert.equals("", service.Message);
    }

}
