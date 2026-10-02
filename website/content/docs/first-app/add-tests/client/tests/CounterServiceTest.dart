@Test(HelloTestSetup, {"partition":"tests"})
class CounterServiceTest {
  @Fake
  GreetingClient? greetingClient;

  Future<void> testIncrementAddsOne() async {
    final firstClient = GreetingClient(behaviors!);
    greetingClient = firstClient;
    final counter = construct<CounterService>(CounterService);
    greetingClient = GreetingClient(behaviors!);
    Assert.isTrue(identical(firstClient, counter.greetingClient));
    counter.increment();
    Assert.equals(1, counter.count);
  }

  Future<void> testAskServerSavesTheMessage() async {
    greetingClient = GreetingClient(behaviors!);
    final counter = construct<CounterService>(CounterService);
    await counter.askServer();
    Assert.equals('Hi', counter.message);
  }
}
