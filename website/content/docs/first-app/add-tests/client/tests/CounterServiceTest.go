// @Test(HelloTestSetup, {"partition":"tests"})
type CounterServiceTest struct {
	behaviors *BehaviorContext
	// @Fake
	greetingClient *GreetingClient
}

func (test *CounterServiceTest) TestIncrementAddsOne() {
	counter := &CounterService{}
	test.construct(counter)
	counter.Increment()
	Assert.Equals(1, counter.Count)
}

func (test *CounterServiceTest) TestAskServerSavesTheMessage() {
	counter := &CounterService{}
	test.construct(counter)
	Assert.Equals(nil, counter.AskServer())
	Assert.Equals("Hi", counter.Message)
}
