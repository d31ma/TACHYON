// @Test(HelloTestSetup, {"partition":"tests"})
type CounterServiceTest struct {
	behaviors *BehaviorContext
}

func (test *CounterServiceTest) TestIncrementAddsOne() {
	counter := &CounterService{}
	counter.Increment()
	Assert.Equals(1, counter.Count)
}
