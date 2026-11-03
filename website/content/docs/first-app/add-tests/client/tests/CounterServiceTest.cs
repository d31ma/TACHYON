[Test(typeof(HelloTestSetup), {"partition":"tests"})]
public sealed class CounterServiceTest
{
    [Fake]
    public GreetingClient _greetingClient = null;

    void testIncrementAddsOne()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(HelloTestSetup), "counter-increment", signal);
        _greetingClient = new GreetingClient(context);
        var counter = construct<CounterService>();
        counter.Increment();
        Assert.equals(1, counter.Count);
    }

    void testAskServerSavesTheTypedMessage()
    {
        using var signal = new BehaviorSignal();
        using var context = Behaviors.create(typeof(HelloTestSetup), "counter-message", signal);
        _greetingClient = new GreetingClient(context);
        var counter = construct<CounterService>();
        var greeting = counter.AskServer().GetAwaiter().GetResult();
        Assert.equals("Hi", greeting.Message);
        Assert.equals("Hi", counter.Message);
    }

    void testConstructedCountersKeepTheirOwnClientContext()
    {
        using var firstSignal = new BehaviorSignal();
        using var secondSignal = new BehaviorSignal();
        using var firstContext = Behaviors.create(typeof(HelloTestSetup), "first-counter", firstSignal);
        using var secondContext = Behaviors.create(typeof(HelloTestSetup), "second-counter", secondSignal);
        _greetingClient = new GreetingClient(firstContext);
        var first = construct<CounterService>();
        first.Increment();
        _greetingClient = new GreetingClient(secondContext);
        var second = construct<CounterService>();
        _greetingClient = null;
        firstContext.Dispose();
        var refused = false;
        try { first.AskServer().GetAwaiter().GetResult(); }
        catch (BehaviorError error) { refused = error.Code == "CANCELLED"; }
        Assert.isTrue(refused);
        Assert.equals("", first.Message);
        Assert.equals("Hi", second.AskServer().GetAwaiter().GetResult().Message);
        Assert.equals("Hi", second.Message);
        Assert.equals(1, first.Count);
        Assert.equals(0, second.Count);
    }
}
