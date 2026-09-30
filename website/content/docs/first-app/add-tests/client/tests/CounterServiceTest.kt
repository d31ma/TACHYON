@Test(HelloTestSetup::class, {"partition":"tests"})
class CounterServiceTest {
    fun testIncrementAddsOne() {
        val counter = CounterService()
        counter.increment()
        Assert.equals(1, counter.count)
    }
}
