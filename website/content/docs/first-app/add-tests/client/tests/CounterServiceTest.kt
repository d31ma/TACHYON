@Test(HelloTestSetup::class, {"partition":"tests"})
class CounterServiceTest {
  @Fake
  var greetingClient: GreetingClient? = null

  fun testIncrementAddsOne() {
    val counter = construct(CounterService::class)
    counter.increment()
    Assert.equals(1, counter.count)
  }

  suspend fun testAskServerSavesTheMessage() {
    val counter = construct(CounterService::class)
    counter.askServer()
    Assert.equals("Hi", counter.message)
  }
}
