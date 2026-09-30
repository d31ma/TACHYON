@Test(HelloTestSetup, {"partition":"tests"})
class CounterServiceTest {
  Future<void> testIncrementAddsOne() async {
    final counter = CounterService();
    counter.increment();
    Assert.equals(1, counter.count);
  }
}
