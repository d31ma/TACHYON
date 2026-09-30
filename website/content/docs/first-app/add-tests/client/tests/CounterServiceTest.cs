[Test(typeof(HelloTestSetup), {"partition":"tests"})]
sealed class CounterServiceTest {
    void testIncrementAddsOne() {
        var counter = new CounterService();
        counter.Increment();
        Assert.equals(1, counter.Count);
    }
}
