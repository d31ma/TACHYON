@Test(HelloTestSetup.class, {"partition":"tests"})
class CounterServiceTest {
    void testIncrementAddsOne() {
        CounterService counter = new CounterService();
        counter.increment();
        Assert.equals(1, counter.count);
    }
}
