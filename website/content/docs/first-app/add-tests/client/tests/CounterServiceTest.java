@Test(HelloTestSetup.class, {"partition":"tests"})
class CounterServiceTest {
    @Fake
    GreetingClient greetingClient = null;

    void testIncrementAddsOne() {
        CounterService counter = construct(CounterService.class);
        counter.increment();
        Assert.equals(1, counter.count);
    }

    void testAskServerSavesTheMessage() {
        CounterService counter = construct(CounterService.class);
        counter.askServer();
        Assert.equals("Hi", counter.message);
    }
}
