@Test(HelloTestSetup, {"partition":"tests"})
class CounterServiceTest:
    def test_increment_adds_one(self):
        counter = CounterService()
        counter.increment()
        Assert.equals(1, counter.count)
